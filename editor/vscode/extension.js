// Elomar-Lang for VS Code.
//
// Language features come from the language server `elomar lsp`, started by
// this extension and spoken to over JSON-RPC on its standard input/output:
// diagnostics while typing, hover, go to definition, the outline and
// completion. Formatting uses `elomar fmt --stdin`, Run and Ctrl+F5 run the
// file in a terminal, and F5 debugs it with `elomar dap`.

'use strict';

const vscode = require('vscode');
const cp = require('child_process');
const path = require('path');

/** @type {vscode.DiagnosticCollection} */
let collection;
/** @type {Server | undefined} */
let server;
let warnedMissing = false;
/** Pending text synchronization, by document URI. */
const timers = new Map();

function settings() {
  return vscode.workspace.getConfiguration('elomar');
}

function executable() {
  return settings().get('path') || 'elomar';
}

/** `en` or `ar`, from the setting or VS Code's display language. */
function messageLanguage() {
  const chosen = settings().get('language');
  if (chosen === 'en' || chosen === 'ar') {
    return chosen;
  }
  return vscode.env.language.toLowerCase().startsWith('ar') ? 'ar' : 'en';
}

function isElomarFile(doc) {
  return doc.languageId === 'elomar' && doc.uri.scheme === 'file';
}

function samePath(a, b) {
  const norm = (p) => path.resolve(p);
  return process.platform === 'win32'
    ? norm(a).toLowerCase() === norm(b).toLowerCase()
    : norm(a) === norm(b);
}

function warnMissing() {
  if (warnedMissing) {
    return;
  }
  warnedMissing = true;
  vscode.window
    .showWarningMessage(
      'Elomar: the `elomar` command was not found. Install it with ' +
        '`cargo install --path crates/elomar-cli`, or set `elomar.path`.',
      'Open Settings',
    )
    .then((choice) => {
      if (choice) {
        vscode.commands.executeCommand('workbench.action.openSettings', 'elomar.path');
      }
    });
}

// ----- the language server connection ------------------------------------------

class Server {
  constructor() {
    this.nextId = 0;
    this.pending = new Map();
    this.buffer = Buffer.alloc(0);
    /** Documents the server has open, by URI string, with their version. */
    this.open = new Map();
    this.failed = false;
    this.child = cp.spawn(executable(), ['--lang', messageLanguage(), 'lsp'], {
      stdio: ['pipe', 'pipe', 'ignore'],
    });
    this.child.on('error', (err) => {
      this.failed = true;
      if (err.code === 'ENOENT') {
        warnMissing();
      }
      this.rejectAll();
    });
    this.child.on('exit', () => {
      this.failed = true;
      this.rejectAll();
    });
    this.child.stdin.on('error', () => {});
    this.child.stdout.on('data', (chunk) => this.receive(chunk));
    this.ready = this.request('initialize', {
      processId: process.pid,
      rootUri: null,
      capabilities: {},
    }).then(() => this.notify('initialized', {}));
  }

  rejectAll() {
    for (const { resolve } of this.pending.values()) {
      resolve(null);
    }
    this.pending.clear();
  }

  send(message) {
    if (this.failed) {
      return;
    }
    const body = Buffer.from(JSON.stringify(message), 'utf8');
    this.child.stdin.write(`Content-Length: ${body.length}\r\n\r\n`);
    this.child.stdin.write(body);
  }

  request(method, params) {
    if (this.failed) {
      return Promise.resolve(null);
    }
    const id = ++this.nextId;
    return new Promise((resolve) => {
      this.pending.set(id, { resolve });
      this.send({ jsonrpc: '2.0', id, method, params });
    });
  }

  notify(method, params) {
    this.send({ jsonrpc: '2.0', method, params });
  }

  receive(chunk) {
    this.buffer = Buffer.concat([this.buffer, chunk]);
    for (;;) {
      const headerEnd = this.buffer.indexOf('\r\n\r\n');
      if (headerEnd < 0) {
        return;
      }
      const header = this.buffer.slice(0, headerEnd).toString('ascii');
      const match = /Content-Length:\s*(\d+)/i.exec(header);
      const length = match ? Number(match[1]) : 0;
      const start = headerEnd + 4;
      if (this.buffer.length < start + length) {
        return;
      }
      const body = this.buffer.slice(start, start + length).toString('utf8');
      this.buffer = this.buffer.slice(start + length);
      let message;
      try {
        message = JSON.parse(body);
      } catch {
        continue;
      }
      if (message.id !== undefined && this.pending.has(message.id)) {
        this.pending.get(message.id).resolve(message.result === undefined ? null : message.result);
        this.pending.delete(message.id);
      } else if (message.method === 'textDocument/publishDiagnostics') {
        publish(message.params);
      }
    }
  }

  /** Sends the document's current text if the server has an older one. */
  sync(doc) {
    if (!isElomarFile(doc)) {
      return;
    }
    const key = doc.uri.toString();
    const known = this.open.get(key);
    if (known === doc.version) {
      return;
    }
    if (known === undefined) {
      this.notify('textDocument/didOpen', {
        textDocument: { uri: key, languageId: 'elomar', version: doc.version, text: doc.getText() },
      });
    } else {
      this.notify('textDocument/didChange', {
        textDocument: { uri: key, version: doc.version },
        contentChanges: [{ text: doc.getText() }],
      });
    }
    this.open.set(key, doc.version);
  }

  close(doc) {
    const key = doc.uri.toString();
    if (this.open.delete(key)) {
      this.notify('textDocument/didClose', { textDocument: { uri: key } });
    }
  }

  stop() {
    if (!this.failed) {
      this.request('shutdown', null).then(() => this.notify('exit', null));
      setTimeout(() => this.child.kill(), 1000);
    }
  }
}

/** The running server, started (or restarted after a crash) on demand. */
function getServer() {
  if (!server || server.failed) {
    server = new Server();
  }
  return server;
}

function schedule(doc, delayMs) {
  if (!isElomarFile(doc)) {
    return;
  }
  const key = doc.uri.toString();
  clearTimeout(timers.get(key));
  timers.set(
    key,
    setTimeout(() => {
      timers.delete(key);
      if (!doc.isClosed) {
        getServer().sync(doc);
      }
    }, delayMs),
  );
}

/** Makes sure the server has the document's latest text before a request. */
async function current(doc) {
  const s = getServer();
  await s.ready;
  clearTimeout(timers.get(doc.uri.toString()));
  s.sync(doc);
  return s;
}

function toRange(r) {
  return new vscode.Range(r.start.line, r.start.character, r.end.line, r.end.character);
}

function toPosition(p) {
  return { line: p.line, character: p.character };
}

function publish(params) {
  const uri = vscode.Uri.parse(params.uri);
  const list = (params.diagnostics || []).map((d) => {
    const diagnostic = new vscode.Diagnostic(
      toRange(d.range),
      d.message,
      d.severity === 2 ? vscode.DiagnosticSeverity.Warning : vscode.DiagnosticSeverity.Error,
    );
    diagnostic.source = 'elomar';
    diagnostic.code = d.code;
    diagnostic.relatedInformation = (d.relatedInformation || []).map(
      (r) => new vscode.DiagnosticRelatedInformation(
        new vscode.Location(vscode.Uri.parse(r.location.uri), toRange(r.location.range)),
        r.message,
      ),
    );
    return diagnostic;
  });
  collection.set(uri, list);
}

// ----- providers -------------------------------------------------------------------

const hoverProvider = {
  async provideHover(doc, position) {
    const s = await current(doc);
    const r = await s.request('textDocument/hover', {
      textDocument: { uri: doc.uri.toString() },
      position: toPosition(position),
    });
    if (!r || !r.contents) {
      return undefined;
    }
    return new vscode.Hover(
      new vscode.MarkdownString(r.contents.value),
      r.range ? toRange(r.range) : undefined,
    );
  },
};

const definitionProvider = {
  async provideDefinition(doc, position) {
    const s = await current(doc);
    const r = await s.request('textDocument/definition', {
      textDocument: { uri: doc.uri.toString() },
      position: toPosition(position),
    });
    if (!r || !r.uri) {
      return undefined;
    }
    return new vscode.Location(vscode.Uri.parse(r.uri), toRange(r.range));
  },
};

function toSymbol(s) {
  const symbol = new vscode.DocumentSymbol(
    s.name,
    '',
    // LSP symbol kinds start at 1, VS Code's at 0.
    Math.max(0, (s.kind || 13) - 1),
    toRange(s.range),
    toRange(s.selectionRange),
  );
  symbol.children = (s.children || []).map(toSymbol);
  return symbol;
}

const symbolProvider = {
  async provideDocumentSymbols(doc) {
    const s = await current(doc);
    const r = await s.request('textDocument/documentSymbol', {
      textDocument: { uri: doc.uri.toString() },
    });
    return (r || []).map(toSymbol);
  },
};

const completionProvider = {
  async provideCompletionItems(doc, position) {
    const s = await current(doc);
    const r = await s.request('textDocument/completion', {
      textDocument: { uri: doc.uri.toString() },
      position: toPosition(position),
    });
    return (r || []).map((c) => {
      // LSP completion kinds start at 1, VS Code's at 0.
      const item = new vscode.CompletionItem(c.label, Math.max(0, (c.kind || 1) - 1));
      if (c.detail) {
        item.detail = c.detail;
      }
      return item;
    });
  },
};

// ----- run and format ----------------------------------------------------------------

async function runCurrentFile() {
  const editor = vscode.window.activeTextEditor;
  if (!editor || !isElomarFile(editor.document)) {
    vscode.window.showInformationMessage('Elomar: open a .elo file to run it.');
    return;
  }
  await runFile(editor.document.uri.fsPath);
}

/** Saves the file if it is open with changes, then runs it in a terminal. */
async function runFile(file) {
  const doc = vscode.workspace.textDocuments.find((d) => samePath(d.uri.fsPath, file));
  if (doc && doc.isDirty) {
    await doc.save();
  }
  const uri = vscode.Uri.file(file);
  const task = new vscode.Task(
    { type: 'elomar', file },
    vscode.workspace.getWorkspaceFolder(uri) || vscode.TaskScope.Workspace,
    `run ${path.basename(file)}`,
    'elomar',
    new vscode.ProcessExecution(executable(), ['--lang', messageLanguage(), 'run', file], {
      cwd: path.dirname(file),
    }),
  );
  task.presentationOptions = { reveal: vscode.TaskRevealKind.Always, clear: true };
  await vscode.tasks.executeTask(task);
}

/**
 * F5 debugs the file with breakpoints; Ctrl+F5 (run without debugging) runs
 * it in a terminal, where `input(...)` reads from the keyboard. Without a
 * launch.json, the open .elo file is used.
 */
const launchProvider = {
  async resolveDebugConfiguration(_folder, config) {
    let file = config.program;
    if (!file || file.includes('${')) {
      const editor = vscode.window.activeTextEditor;
      file = editor && isElomarFile(editor.document) ? editor.document.uri.fsPath : undefined;
    }
    if (!file) {
      vscode.window.showInformationMessage('Elomar: open a .elo file to run it.');
      return undefined;
    }
    if (config.noDebug) {
      runFile(file);
      return undefined;
    }
    const doc = vscode.workspace.textDocuments.find((d) => samePath(d.uri.fsPath, file));
    if (doc && doc.isDirty) {
      await doc.save();
    }
    return {
      ...config,
      type: 'elomar',
      request: 'launch',
      name: config.name || `Debug ${path.basename(file)}`,
      program: file,
    };
  },
};

/** Starts `elomar dap` for each debug session. */
const adapterFactory = {
  createDebugAdapterDescriptor(session) {
    const program = session.configuration.program;
    return new vscode.DebugAdapterExecutable(executable(), ['--lang', messageLanguage(), 'dap'], {
      cwd: program ? path.dirname(program) : undefined,
    });
  },
};

/** Format Document / format on save, via `elomar fmt --stdin`. */
const formatter = {
  provideDocumentFormattingEdits(doc) {
    return new Promise((resolve) => {
      const text = doc.getText();
      const child = cp.spawn(executable(), ['--lang', messageLanguage(), 'fmt', '--stdin'], {
        cwd: path.dirname(doc.uri.fsPath),
      });
      let output = '';
      child.stdout.setEncoding('utf8');
      child.stdout.on('data', (chunk) => {
        output += chunk;
      });
      child.on('error', () => resolve([]));
      child.on('close', (code) => {
        // Code with syntax errors is left as it is; the errors are already shown.
        if (code !== 0 || output === text) {
          resolve([]);
          return;
        }
        const all = new vscode.Range(doc.positionAt(0), doc.positionAt(text.length));
        resolve([vscode.TextEdit.replace(all, output)]);
      });
      child.stdin.on('error', () => {});
      child.stdin.end(text, 'utf8');
    });
  },
};

function activate(context) {
  collection = vscode.languages.createDiagnosticCollection('elomar');
  const selector = { language: 'elomar', scheme: 'file' };
  context.subscriptions.push(
    collection,
    vscode.workspace.onDidOpenTextDocument((doc) => schedule(doc, 0)),
    vscode.workspace.onDidSaveTextDocument((doc) => schedule(doc, 0)),
    vscode.workspace.onDidChangeTextDocument((event) => {
      if (settings().get('checkOnType')) {
        schedule(event.document, 300);
      }
    }),
    vscode.workspace.onDidCloseTextDocument((doc) => {
      clearTimeout(timers.get(doc.uri.toString()));
      if (server) {
        server.close(doc);
      }
      collection.delete(doc.uri);
    }),
    vscode.workspace.onDidChangeConfiguration((event) => {
      if (event.affectsConfiguration('elomar')) {
        // Restart the server with the new path or language.
        warnedMissing = false;
        if (server) {
          server.stop();
          server = undefined;
        }
        collection.clear();
        vscode.workspace.textDocuments.forEach((doc) => schedule(doc, 0));
      }
    }),
    vscode.commands.registerCommand('elomar.run', runCurrentFile),
    vscode.commands.registerCommand('elomar.check', () => {
      const editor = vscode.window.activeTextEditor;
      if (editor) {
        schedule(editor.document, 0);
      }
    }),
    vscode.languages.registerDocumentFormattingEditProvider(selector, formatter),
    vscode.languages.registerHoverProvider(selector, hoverProvider),
    vscode.languages.registerDefinitionProvider(selector, definitionProvider),
    vscode.languages.registerDocumentSymbolProvider(selector, symbolProvider),
    vscode.languages.registerCompletionItemProvider(selector, completionProvider, '.'),
    vscode.debug.registerDebugConfigurationProvider('elomar', launchProvider),
    vscode.debug.registerDebugAdapterDescriptorFactory('elomar', adapterFactory),
  );
  vscode.workspace.textDocuments.forEach((doc) => schedule(doc, 0));
}

function deactivate() {
  for (const timer of timers.values()) {
    clearTimeout(timer);
  }
  if (server) {
    server.stop();
  }
}

module.exports = { activate, deactivate };
