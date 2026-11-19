// Elomar-Lang for VS Code: live diagnostics and a Run command.
//
// Diagnostics come from `elomar check --format json --stdin <file>`, which
// checks the editor's current (possibly unsaved) text and reports problems
// in that file and in the modules it imports.

'use strict';

const vscode = require('vscode');
const cp = require('child_process');
const path = require('path');

/** @type {vscode.DiagnosticCollection} */
let collection;
let warnedMissing = false;
/** Pending checks, by document URI. */
const timers = new Map();
/** Files that received diagnostics from each document's last check. */
const reportedFiles = new Map();

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

function schedule(doc, delayMs) {
  if (!isElomarFile(doc)) {
    return;
  }
  const key = doc.uri.toString();
  clearTimeout(timers.get(key));
  timers.set(key, setTimeout(() => check(doc), delayMs));
}

function check(doc) {
  if (doc.isClosed) {
    return;
  }
  const file = doc.uri.fsPath;
  const version = doc.version;
  const child = cp.spawn(
    executable(),
    ['--lang', messageLanguage(), 'check', '--format', 'json', '--stdin', file],
    { cwd: path.dirname(file) },
  );
  let output = '';
  child.stdout.setEncoding('utf8');
  child.stdout.on('data', (chunk) => {
    output += chunk;
  });
  child.on('error', (err) => {
    if (err.code === 'ENOENT' && !warnedMissing) {
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
  });
  child.on('close', () => {
    // A newer check is already scheduled for edited text.
    if (doc.isClosed || doc.version !== version) {
      return;
    }
    let result;
    try {
      result = JSON.parse(output);
    } catch {
      return;
    }
    publish(doc, result.diagnostics || []);
  });
  child.stdin.on('error', () => {});
  child.stdin.end(doc.getText(), 'utf8');
}

function toRange(r) {
  return new vscode.Range(r.start.line, r.start.character, r.end.line, r.end.character);
}

function uriFor(file, doc) {
  return samePath(file, doc.uri.fsPath) ? doc.uri : vscode.Uri.file(file);
}

function publish(doc, items) {
  const byFile = new Map();
  byFile.set(doc.uri.toString(), { uri: doc.uri, list: [] });
  for (const item of items) {
    const uri = uriFor(item.file, doc);
    const key = uri.toString();
    if (!byFile.has(key)) {
      byFile.set(key, { uri, list: [] });
    }
    const message = item.hint ? `${item.message}\n${item.hint}` : item.message;
    const diagnostic = new vscode.Diagnostic(
      toRange(item.range),
      message,
      item.severity === 'warning'
        ? vscode.DiagnosticSeverity.Warning
        : vscode.DiagnosticSeverity.Error,
    );
    diagnostic.source = 'elomar';
    diagnostic.code = item.code;
    diagnostic.relatedInformation = (item.related || []).map(
      (r) => new vscode.DiagnosticRelatedInformation(
        new vscode.Location(uriFor(r.file, doc), toRange(r.range)),
        r.message,
      ),
    );
    byFile.get(key).list.push(diagnostic);
  }

  // Clear files this document reported on before but not now.
  const docKey = doc.uri.toString();
  for (const old of reportedFiles.get(docKey) || []) {
    if (!byFile.has(old)) {
      collection.delete(vscode.Uri.parse(old));
    }
  }
  for (const { uri, list } of byFile.values()) {
    collection.set(uri, list);
  }
  reportedFiles.set(docKey, [...byFile.keys()]);
}

function clear(doc) {
  const key = doc.uri.toString();
  clearTimeout(timers.get(key));
  timers.delete(key);
  for (const file of reportedFiles.get(key) || []) {
    collection.delete(vscode.Uri.parse(file));
  }
  reportedFiles.delete(key);
}

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
 * F5 / Ctrl+F5: there is no debugger for Elomar yet, so a launch runs the
 * file instead. Returning `undefined` ends the launch without starting a
 * debug session (and without asking for a debugger extension).
 */
const runOnLaunch = {
  resolveDebugConfiguration(_folder, config) {
    let file = config.program;
    if (!file || file.includes('${')) {
      const editor = vscode.window.activeTextEditor;
      file = editor && isElomarFile(editor.document) ? editor.document.uri.fsPath : undefined;
    }
    if (!file) {
      vscode.window.showInformationMessage('Elomar: open a .elo file to run it.');
      return undefined;
    }
    runFile(file);
    return undefined;
  },
};

function activate(context) {
  collection = vscode.languages.createDiagnosticCollection('elomar');
  context.subscriptions.push(
    collection,
    vscode.workspace.onDidOpenTextDocument((doc) => schedule(doc, 0)),
    vscode.workspace.onDidSaveTextDocument((doc) => schedule(doc, 0)),
    vscode.workspace.onDidChangeTextDocument((event) => {
      if (settings().get('checkOnType')) {
        schedule(event.document, 400);
      }
    }),
    vscode.workspace.onDidCloseTextDocument(clear),
    vscode.workspace.onDidChangeConfiguration((event) => {
      if (event.affectsConfiguration('elomar')) {
        warnedMissing = false;
        vscode.workspace.textDocuments.forEach((doc) => schedule(doc, 0));
      }
    }),
    vscode.commands.registerCommand('elomar.run', runCurrentFile),
    vscode.debug.registerDebugConfigurationProvider('elomar', runOnLaunch),
    vscode.commands.registerCommand('elomar.check', () => {
      const editor = vscode.window.activeTextEditor;
      if (editor) {
        schedule(editor.document, 0);
      }
    }),
  );
  vscode.workspace.textDocuments.forEach((doc) => schedule(doc, 0));
}

function deactivate() {
  for (const timer of timers.values()) {
    clearTimeout(timer);
  }
}

module.exports = { activate, deactivate };
