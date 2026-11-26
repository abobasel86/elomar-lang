//! `elomar lsp`: a Language Server Protocol server on standard input and
//! output, for any editor (VS Code, Neovim, Helix, Zed, ...).
//!
//! Supports diagnostics while typing, hover (types and signatures), go to
//! definition (also into imported modules), document symbols (outline),
//! completion (names, and members after `.`) and formatting. Documents are
//! synchronized in full on every change. Hover and completion keep using
//! the last analysis that succeeded while the text has syntax errors.

use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use elomar_ast::{Item, TypeExprKind};
use elomar_diagnostics::{Lang, SourceFile, SourceMap, Span};
use elomar_types::IdeInfo;
use serde_json::{Value, json};

/// The result of checking one document.
struct Analysis {
    info: IdeInfo,
    sources: SourceMap,
    paths: Vec<PathBuf>,
    /// The document's text when it was analyzed.
    text: String,
}

struct Doc {
    path: PathBuf,
    text: String,
    /// The latest analysis without syntax errors.
    analysis: Option<Analysis>,
    /// Files this document's diagnostics were published for.
    reported: Vec<String>,
}

struct Server {
    lang: Lang,
    docs: HashMap<String, Doc>,
    out: std::io::Stdout,
}

pub(crate) fn serve(lang: Lang) -> ExitCode {
    let mut server = Server {
        lang,
        docs: HashMap::new(),
        out: std::io::stdout(),
    };
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    let mut shutting_down = false;
    loop {
        let Some(message) = read_message(&mut input) else {
            return ExitCode::from(if shutting_down { 0 } else { 1 });
        };
        let method = message["method"].as_str().unwrap_or_default().to_string();
        let id = message.get("id").cloned();
        let params = &message["params"];
        let result = match method.as_str() {
            "initialize" => Some(capabilities()),
            "shutdown" => {
                shutting_down = true;
                Some(Value::Null)
            }
            "exit" => return ExitCode::from(if shutting_down { 0 } else { 1 }),
            "textDocument/didOpen" => {
                let doc = &params["textDocument"];
                server.update(str_of(&doc["uri"]), str_of(&doc["text"]).to_string());
                None
            }
            "textDocument/didChange" => {
                // Full synchronization: the last change holds the whole text.
                if let Some(text) = params["contentChanges"]
                    .as_array()
                    .and_then(|c| c.last())
                    .and_then(|c| c["text"].as_str())
                {
                    server.update(str_of(&params["textDocument"]["uri"]), text.to_string());
                }
                None
            }
            "textDocument/didSave" => None,
            "textDocument/didClose" => {
                server.close(str_of(&params["textDocument"]["uri"]));
                None
            }
            "textDocument/hover" => Some(server.hover(params)),
            "textDocument/definition" => Some(server.definition(params)),
            "textDocument/documentSymbol" => Some(server.symbols(params)),
            "textDocument/completion" => Some(server.completion(params)),
            "textDocument/formatting" => Some(server.formatting(params)),
            "textDocument/references" => Some(server.references(params)),
            "textDocument/prepareRename" => Some(server.prepare_rename(params)),
            "textDocument/rename" => match server.rename(params) {
                Ok(edit) => Some(edit),
                Err(message) => {
                    server.send(&json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": { "code": -32803, "message": message },
                    }));
                    None
                }
            },
            "textDocument/signatureHelp" => Some(server.signature_help(params)),
            _ => {
                if id.is_some() && !method.is_empty() {
                    // Unknown request: "method not found".
                    server.send(&json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": { "code": -32601, "message": format!("unsupported: {method}") },
                    }));
                }
                None
            }
        };
        if let (Some(id), Some(result)) = (id, result) {
            server.send(&json!({ "jsonrpc": "2.0", "id": id, "result": result }));
        }
    }
}

fn capabilities() -> Value {
    json!({
        "capabilities": {
            "textDocumentSync": { "openClose": true, "change": 1, "save": false },
            "hoverProvider": true,
            "definitionProvider": true,
            "documentSymbolProvider": true,
            "documentFormattingProvider": true,
            "completionProvider": { "triggerCharacters": ["."] },
            "referencesProvider": true,
            "renameProvider": { "prepareProvider": true },
            "signatureHelpProvider": { "triggerCharacters": ["(", ","] },
        },
        "serverInfo": { "name": "elomar", "version": env!("CARGO_PKG_VERSION") },
    })
}

fn str_of(v: &Value) -> &str {
    v.as_str().unwrap_or_default()
}

/// Reads one `Content-Length`-framed JSON-RPC message.
fn read_message(input: &mut impl BufRead) -> Option<Value> {
    let mut length = None;
    loop {
        let mut line = String::new();
        if input.read_line(&mut line).ok()? == 0 {
            return None;
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some(v) = line.strip_prefix("Content-Length:") {
            length = v.trim().parse::<usize>().ok();
        }
    }
    let mut body = vec![0; length?];
    input.read_exact(&mut body).ok()?;
    serde_json::from_slice(&body).ok()
}

// ----- URIs and positions -------------------------------------------------------

/// `file:///d%3A/x/main.elo` → `d:/x/main.elo` (or `/x/main.elo`).
pub(crate) fn uri_to_path(uri: &str) -> PathBuf {
    let rest = uri.strip_prefix("file://").unwrap_or(uri);
    let mut bytes = Vec::with_capacity(rest.len());
    let raw = rest.as_bytes();
    let mut i = 0;
    while i < raw.len() {
        if raw[i] == b'%'
            && i + 2 < raw.len()
            && let Ok(b) = u8::from_str_radix(&rest[i + 1..i + 3], 16)
        {
            bytes.push(b);
            i += 3;
            continue;
        }
        bytes.push(raw[i]);
        i += 1;
    }
    let decoded = String::from_utf8_lossy(&bytes).into_owned();
    // `/d:/x` on Windows.
    let trimmed = match decoded.as_bytes() {
        [b'/', drive, b':', ..] if drive.is_ascii_alphabetic() => decoded[1..].to_string(),
        _ => decoded,
    };
    PathBuf::from(trimmed)
}

pub(crate) fn path_to_uri(path: &Path) -> String {
    let text = path.to_string_lossy().replace('\\', "/");
    let mut out = String::from("file://");
    if !text.starts_with('/') {
        out.push('/');
    }
    for b in text.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' | b':' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// LSP position (0-based line, UTF-16 column) → byte offset in `text`.
fn offset_at(text: &str, line: usize, character: usize) -> usize {
    let mut start = 0;
    for _ in 0..line {
        match text[start..].find('\n') {
            Some(i) => start += i + 1,
            None => return text.len(),
        }
    }
    let end = text[start..].find('\n').map_or(text.len(), |i| start + i);
    let mut units = 0;
    for (i, c) in text[start..end].char_indices() {
        if units >= character {
            return start + i;
        }
        units += c.len_utf16();
    }
    end
}

fn position_of(params: &Value) -> (usize, usize) {
    let p = &params["position"];
    (
        p["line"].as_u64().unwrap_or(0) as usize,
        p["character"].as_u64().unwrap_or(0) as usize,
    )
}

fn range_json(r: &Value) -> Value {
    r.clone()
}

/// The file and LSP range of a span from an analysis.
fn locate(span: Span, sources: &SourceMap, paths: &[PathBuf]) -> (PathBuf, Value) {
    let index = sources.index_at(span.start as usize);
    let (file, base) = sources.file_at(span.start as usize);
    let start = file.position_utf16(span.start as usize - base);
    let end = file.position_utf16(
        (span.end as usize)
            .saturating_sub(base)
            .max(span.start as usize - base),
    );
    let path = paths
        .get(index)
        .cloned()
        .unwrap_or_else(|| PathBuf::from(&file.name));
    (
        path,
        json!({
            "start": { "line": start.0, "character": start.1 },
            "end": { "line": end.0, "character": end.1 },
        }),
    )
}

/// The smallest span in `spans` that contains `offset` within the entry
/// file (base 0, length `len`).
fn innermost<T>(
    items: impl Iterator<Item = (Span, T)>,
    offset: usize,
    len: usize,
) -> Option<(Span, T)> {
    items
        .filter(|(s, _)| {
            let (a, b) = (s.start as usize, s.end as usize);
            b <= len && a <= offset && offset <= b && a < b
        })
        .min_by_key(|(s, _)| s.end - s.start)
}

// ----- the server -------------------------------------------------------------------

impl Server {
    fn send(&mut self, message: &Value) {
        let body = message.to_string();
        let mut out = self.out.lock();
        let _ = write!(out, "Content-Length: {}\r\n\r\n{body}", body.len());
        let _ = out.flush();
    }

    fn notify(&mut self, method: &str, params: Value) {
        self.send(&json!({ "jsonrpc": "2.0", "method": method, "params": params }));
    }

    fn update(&mut self, uri: &str, text: String) {
        let path = uri_to_path(uri);
        let previous = self.docs.remove(uri);
        let (old_analysis, reported) = match previous {
            Some(d) => (d.analysis, d.reported),
            None => (None, Vec::new()),
        };
        let (diagnostics, analysis) = analyze(&path, &text, self.lang);
        let mut doc = Doc {
            path,
            text,
            analysis: analysis.or(old_analysis),
            reported,
        };
        self.publish(&mut doc, uri, diagnostics);
        self.docs.insert(uri.to_string(), doc);
    }

    fn close(&mut self, uri: &str) {
        if let Some(doc) = self.docs.remove(uri) {
            for file in doc.reported {
                self.notify(
                    "textDocument/publishDiagnostics",
                    json!({ "uri": file, "diagnostics": [] }),
                );
            }
        }
    }

    /// Publishes diagnostics by file, and clears files reported before.
    fn publish(&mut self, doc: &mut Doc, uri: &str, items: Vec<Value>) {
        let mut by_file: Vec<(String, Vec<Value>)> = vec![(uri.to_string(), Vec::new())];
        for item in items {
            let file = path_to_uri(Path::new(str_of(&item["file"])));
            let file = if same_path(Path::new(str_of(&item["file"])), &doc.path) {
                uri.to_string()
            } else {
                file
            };
            let mut message = str_of(&item["message"]).to_string();
            if let Some(hint) = item["hint"].as_str() {
                message.push('\n');
                message.push_str(hint);
            }
            let related: Vec<Value> = item["related"]
                .as_array()
                .map(|r| {
                    r.iter()
                        .map(|n| {
                            json!({
                                "location": {
                                    "uri": path_to_uri(Path::new(str_of(&n["file"]))),
                                    "range": range_json(&n["range"]),
                                },
                                "message": n["message"],
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();
            let diagnostic = json!({
                "range": range_json(&item["range"]),
                "severity": if item["severity"] == "warning" { 2 } else { 1 },
                "code": item["code"],
                "source": "elomar",
                "message": message,
                "relatedInformation": related,
            });
            match by_file.iter_mut().find(|(f, _)| *f == file) {
                Some((_, list)) => list.push(diagnostic),
                None => by_file.push((file, vec![diagnostic])),
            }
        }
        let now: Vec<String> = by_file.iter().map(|(f, _)| f.clone()).collect();
        for old in std::mem::take(&mut doc.reported) {
            if !now.contains(&old) {
                self.notify(
                    "textDocument/publishDiagnostics",
                    json!({ "uri": old, "diagnostics": [] }),
                );
            }
        }
        for (file, list) in by_file {
            self.notify(
                "textDocument/publishDiagnostics",
                json!({ "uri": file, "diagnostics": list }),
            );
        }
        doc.reported = now;
    }

    fn doc_at(&self, params: &Value) -> Option<(&Doc, usize)> {
        let doc = self.docs.get(str_of(&params["textDocument"]["uri"]))?;
        let (line, character) = position_of(params);
        Some((doc, offset_at(&doc.text, line, character)))
    }

    fn hover(&self, params: &Value) -> Value {
        let Some((doc, offset)) = self.doc_at(params) else {
            return Value::Null;
        };
        let Some(a) = doc.analysis.as_ref().filter(|a| a.text == doc.text) else {
            return Value::Null;
        };
        let found = innermost(
            a.info.hovers.iter().map(|h| (h.span, &h.text)),
            offset,
            a.text.len(),
        );
        match found {
            Some((span, text)) => {
                let (_, range) = locate(span, &a.sources, &a.paths);
                json!({
                    "contents": { "kind": "markdown", "value": format!("```elomar\n{text}\n```") },
                    "range": range,
                })
            }
            None => Value::Null,
        }
    }

    fn definition(&self, params: &Value) -> Value {
        let Some((doc, offset)) = self.doc_at(params) else {
            return Value::Null;
        };
        let Some(a) = doc.analysis.as_ref().filter(|a| a.text == doc.text) else {
            return Value::Null;
        };
        let found = innermost(a.info.definitions.iter().copied(), offset, a.text.len());
        match found {
            Some((_, def)) => {
                let (path, range) = locate(def, &a.sources, &a.paths);
                let uri = if same_path(&path, &doc.path) {
                    str_of(&params["textDocument"]["uri"]).to_string()
                } else {
                    path_to_uri(&path)
                };
                json!({ "uri": uri, "range": range })
            }
            None => Value::Null,
        }
    }

    /// The name under the cursor: its definition and every use of it, in
    /// every file of the program, each narrowed to just the name.
    fn occurrences(&self, params: &Value) -> Option<(String, Vec<Place>, &Doc)> {
        let (doc, offset) = self.doc_at(params)?;
        let a = doc.analysis.as_ref().filter(|a| a.text == doc.text)?;
        let defs = &a.info.definitions;
        // A use of the name, or the definition itself.
        let def = match innermost(defs.iter().copied(), offset, a.text.len()) {
            Some((_, def)) => def,
            None => {
                let mut own: Vec<(Span, ())> = defs.iter().map(|(_, d)| (*d, ())).collect();
                own.extend(a.info.hovers.iter().map(|h| (h.span, ())));
                innermost(own.into_iter(), offset, a.text.len())?.0
            }
        };
        let name = name_in(&a.sources, def)?;
        let mut spans = vec![def];
        spans.extend(defs.iter().filter(|(_, d)| *d == def).map(|(u, _)| *u));
        spans.sort_by_key(|s| s.start);
        spans.dedup();
        let mut out = Vec::new();
        for span in spans {
            // `lib.shout`: only `shout` is the name.
            let (file, base) = a.sources.file_at(span.start as usize);
            let text = file
                .text
                .get(span.start as usize - base..span.end as usize - base)?;
            let start = if text == name {
                span.start
            } else if text.ends_with(&format!(".{name}")) {
                span.end - name.len() as u32
            } else {
                continue;
            };
            let narrowed = Span {
                start,
                end: start + name.len() as u32,
            };
            out.push(locate(narrowed, &a.sources, &a.paths));
        }
        Some((name, out, doc))
    }

    fn references(&self, params: &Value) -> Value {
        let Some((_, found, doc)) = self.occurrences(params) else {
            return Value::Null;
        };
        let uri = str_of(&params["textDocument"]["uri"]).to_string();
        Value::Array(
            found
                .into_iter()
                .map(|(path, range)| {
                    let file = if same_path(&path, &doc.path) {
                        uri.clone()
                    } else {
                        path_to_uri(&path)
                    };
                    json!({ "uri": file, "range": range })
                })
                .collect(),
        )
    }

    fn prepare_rename(&self, params: &Value) -> Value {
        let Some((doc, offset)) = self.doc_at(params) else {
            return Value::Null;
        };
        let Some((name, found, _)) = self.occurrences(params) else {
            return Value::Null;
        };
        // The occurrence under the cursor.
        let file = SourceFile::new("", doc.text.clone());
        let cursor = file.position_utf16(offset);
        for (path, range) in found {
            if !same_path(&path, &doc.path) {
                continue;
            }
            let at = |k: &str| {
                (
                    range[k]["line"].as_u64().unwrap_or(0) as usize,
                    range[k]["character"].as_u64().unwrap_or(0) as usize,
                )
            };
            if at("start") <= cursor && cursor <= at("end") {
                return json!({ "range": range, "placeholder": name });
            }
        }
        Value::Null
    }

    fn rename(&self, params: &Value) -> Result<Value, String> {
        let new_name = str_of(&params["newName"]).to_string();
        if !is_identifier(&new_name) {
            return Err(format!(
                "`{new_name}` is not a valid name (letters, digits and _, not starting with a digit, and not a keyword)"
            ));
        }
        let Some((_, found, doc)) = self.occurrences(params) else {
            return Err("there is no name to rename here".into());
        };
        let root = crate::project_root(&doc.path);
        let uri = str_of(&params["textDocument"]["uri"]).to_string();
        let mut changes = serde_json::Map::new();
        for (path, range) in found {
            if !same_path(&path, &doc.path) && !path.starts_with(&root) {
                return Err(format!(
                    "this name is defined in a package (`{}`), which cannot be changed from here",
                    path.display()
                ));
            }
            let file = if same_path(&path, &doc.path) {
                uri.clone()
            } else {
                path_to_uri(&path)
            };
            let edits = changes
                .entry(file)
                .or_insert_with(|| Value::Array(Vec::new()));
            if let Value::Array(list) = edits {
                list.push(json!({ "range": range, "newText": new_name }));
            }
        }
        Ok(json!({ "changes": changes }))
    }

    /// The parameters of the function being called at the cursor, from the
    /// last successful analysis (the call being typed rarely parses).
    fn signature_help(&self, params: &Value) -> Value {
        let Some((doc, offset)) = self.doc_at(params) else {
            return Value::Null;
        };
        let Some(a) = doc.analysis.as_ref() else {
            return Value::Null;
        };
        let Some((callee, active)) = call_at(&doc.text, offset) else {
            return Value::Null;
        };
        // Any hover showing this function's signature.
        let signature = a.info.hovers.iter().map(|h| h.text.as_str()).find(|t| {
            let t = t
                .trim_start_matches("pub ")
                .trim_start_matches("async ")
                .trim_start_matches("fn ");
            let base = t.split('(').next().unwrap_or("");
            let base = base.split('<').next().unwrap_or("");
            base == callee || base.rsplit("::").next() == Some(callee.as_str())
        });
        let Some(signature) = signature else {
            return Value::Null;
        };
        let inside = signature
            .split_once('(')
            .map(|(_, rest)| rest)
            .and_then(|rest| rest.rsplit_once(')').map(|(inner, _)| inner))
            .unwrap_or("");
        let parameters: Vec<Value> = split_top_level(inside)
            .into_iter()
            .filter(|p| !p.is_empty() && *p != "self")
            .map(|p| json!({ "label": p }))
            .collect();
        json!({
            "signatures": [{ "label": signature, "parameters": parameters }],
            "activeSignature": 0,
            "activeParameter": active.min(parameters.len().saturating_sub(1)),
        })
    }

    fn symbols(&self, params: &Value) -> Value {
        match self.docs.get(str_of(&params["textDocument"]["uri"])) {
            Some(doc) => Value::Array(document_symbols(&doc.text)),
            None => Value::Array(Vec::new()),
        }
    }

    fn formatting(&self, params: &Value) -> Value {
        let Some(doc) = self.docs.get(str_of(&params["textDocument"]["uri"])) else {
            return Value::Null;
        };
        match elomar_fmt::format(&doc.text) {
            Ok(formatted) if formatted != doc.text => {
                let file = SourceFile::new("", doc.text.clone());
                let end = file.position_utf16(doc.text.len());
                json!([{
                    "range": {
                        "start": { "line": 0, "character": 0 },
                        "end": { "line": end.0, "character": end.1 },
                    },
                    "newText": formatted,
                }])
            }
            _ => json!([]),
        }
    }

    fn completion(&self, params: &Value) -> Value {
        let Some((doc, offset)) = self.doc_at(params) else {
            return json!([]);
        };
        let before = &doc.text[..offset];
        // The word being typed, and what precedes it.
        let word_start = before
            .char_indices()
            .rev()
            .take_while(|(_, c)| c.is_alphanumeric() || *c == '_')
            .last()
            .map_or(offset, |(i, _)| i);
        let head = &before[..word_start];
        let mut items: Vec<Value> = Vec::new();
        let mut add = |label: &str, kind: u8, detail: Option<&str>| {
            if !items.iter().any(|i| i["label"] == label) {
                items.push(json!({ "label": label, "kind": kind, "detail": detail }));
            }
        };
        if let Some(receiver_end) = head.strip_suffix('.').map(str::len) {
            // `x.` — members of `x`, or functions of a module.
            let receiver_start = head[..receiver_end]
                .char_indices()
                .rev()
                .take_while(|(_, c)| c.is_alphanumeric() || *c == '_')
                .last()
                .map_or(receiver_end, |(i, _)| i);
            let receiver = &head[receiver_start..receiver_end];
            for f in elomar_types::module_functions(receiver) {
                add(f, 3, None);
            }
            if let Some(a) = &doc.analysis {
                // The latest analyzed use of that name before the cursor.
                let hover = a
                    .info
                    .hovers
                    .iter()
                    .filter(|h| {
                        let (s, e) = (h.span.start as usize, h.span.end as usize);
                        e <= a.text.len() && a.text.get(s..e) == Some(receiver) && s <= offset
                    })
                    .max_by_key(|h| h.span.start);
                if let Some(h) = hover {
                    for m in &h.members {
                        add(m, 2, None);
                    }
                }
            }
            return Value::Array(items);
        }
        // Keywords in the language the editor shows messages in first, then
        // in the other (`دالة` and `fn` are the same keyword).
        for &(_, en, ar) in elomar_lexer::KEYWORDS {
            let (first, second) = if self.lang == Lang::Ar {
                (ar, en)
            } else {
                (en, ar)
            };
            add(first, 14, Some(second));
            add(second, 14, Some(first));
        }
        for f in elomar_types::BUILTIN_FUNCTIONS {
            add(f, 3, Some("built-in"));
        }
        if let Some(a) = &doc.analysis {
            for h in &a.info.hovers {
                let (s, e) = (h.span.start as usize, h.span.end as usize);
                if e > a.text.len() {
                    continue;
                }
                let Some(name) = a.text.get(s..e) else {
                    continue;
                };
                let kind = if h.text.starts_with("fn ")
                    || h.text.starts_with("pub ")
                    || h.text.starts_with("async ")
                {
                    3
                } else if h.text.starts_with("type ") || h.text.starts_with("trait ") {
                    22
                } else if s <= offset {
                    6
                } else {
                    continue;
                };
                if name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                    add(name, kind, Some(&h.text));
                }
            }
        }
        Value::Array(items)
    }
}

/// A file and an LSP range in it.
type Place = (PathBuf, Value);

/// The text of `span`, if it is a single name.
fn name_in(sources: &SourceMap, span: Span) -> Option<String> {
    let (file, base) = sources.file_at(span.start as usize);
    let text = file
        .text
        .get(span.start as usize - base..span.end as usize - base)?;
    let name = text.rsplit('.').next().unwrap_or(text);
    is_identifier(name).then(|| name.to_string())
}

/// A name a program could define: what the lexer reads as one identifier.
fn is_identifier(name: &str) -> bool {
    if name.is_empty() || name == "_" {
        return false;
    }
    let (program, diags) = elomar_parser::parse(&format!("fn {name}() {{}}"));
    diags.is_empty()
        && matches!(program.items.as_slice(), [elomar_ast::Item::Fn(f)] if f.name.name == name)
}

/// The function called around `offset` and which argument the cursor is
/// in: `greet("a", |` gives `("greet", 1)`.
fn call_at(text: &str, offset: usize) -> Option<(String, usize)> {
    let before = text.get(..offset.min(text.len()))?;
    let mut depth = 0usize;
    let mut commas = 0usize;
    let mut in_string = false;
    for (i, c) in before.char_indices().rev() {
        match c {
            '"' => in_string = !in_string,
            _ if in_string => {}
            ')' | ']' | '}' => depth += 1,
            '[' | '{' if depth > 0 => depth -= 1,
            '[' | '{' => return None,
            '(' if depth > 0 => depth -= 1,
            '(' => {
                let name: String = before[..i]
                    .chars()
                    .rev()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect();
                return (!name.is_empty()).then_some((name, commas));
            }
            ',' if depth == 0 => commas += 1,
            _ => {}
        }
    }
    None
}

/// `a: Int, b: List<Map<String, Int>>` → the two parameters.
fn split_top_level(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut start = 0;
    for (i, c) in s.char_indices() {
        match c {
            '<' | '(' | '[' | '{' => depth += 1,
            '>' | ')' | ']' | '}' => depth -= 1,
            ',' if depth == 0 => {
                out.push(s[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(s[start..].trim());
    out
}

fn same_path(a: &Path, b: &Path) -> bool {
    let norm = |p: &Path| {
        let s = std::path::absolute(p)
            .unwrap_or_else(|_| p.to_path_buf())
            .to_string_lossy()
            .replace('\\', "/");
        if cfg!(windows) { s.to_lowercase() } else { s }
    };
    norm(a) == norm(b)
}

/// Checks a document (with the modules it imports) like
/// `elomar check --format json --stdin`.
fn analyze(path: &Path, text: &str, lang: Lang) -> (Vec<Value>, Option<Analysis>) {
    let root = crate::project_root(path);
    let cloud_models = crate::load_ai_runtime(path)
        .map(|ai| ai.cloud_models)
        .unwrap_or_default();
    let loaded = match elomar_loader::load_with(path, &root, Some(text.to_string())) {
        Ok(l) => l,
        Err(_) => return (Vec::new(), None),
    };
    let mut program = loaded.program;
    let mut diags = loaded.diagnostics;
    let mut analysis = None;
    if diags.iter().all(|d| !d.is_error()) {
        let (more, info) = elomar_types::analyze(
            &mut program,
            elomar_types::CheckOptions {
                require_main: false,
                cloud_models,
            },
        );
        diags.extend(more);
        analysis = Some(Analysis {
            info,
            sources: loaded.sources.clone(),
            paths: loaded.paths.clone(),
            text: text.to_string(),
        });
    }
    let items = diags
        .iter()
        .map(|d| crate::editor::diagnostic_json(d, &loaded.sources, &loaded.paths, lang))
        .collect();
    (items, analysis)
}

/// The outline of a document: its items, with enum variants and impl
/// methods as children.
fn document_symbols(text: &str) -> Vec<Value> {
    let (program, _) = elomar_parser::parse(text);
    let file = SourceFile::new("", text.to_string());
    let range = |s: Span| {
        let a = file.position_utf16(s.start as usize);
        let b = file.position_utf16(s.end as usize);
        json!({
            "start": { "line": a.0, "character": a.1 },
            "end": { "line": b.0, "character": b.1 },
        })
    };
    let symbol = |name: &str, kind: u8, span: Span, sel: Span, children: Vec<Value>| {
        json!({
            "name": name,
            "kind": kind,
            "range": range(span),
            "selectionRange": range(sel),
            "children": children,
        })
    };
    let mut out = Vec::new();
    for item in &program.items {
        match item {
            Item::Fn(f) => out.push(symbol(&f.name.name, 12, f.span, f.name.span, vec![])),
            Item::Type(t) => out.push(symbol(&t.name.name, 23, t.span, t.name.span, vec![])),
            Item::Const(c) => {
                out.push(symbol(
                    &c.name.name,
                    14,
                    c.name.span.to(c.value.span),
                    c.name.span,
                    vec![],
                ));
            }
            Item::Enum(e) => {
                let children = e
                    .variants
                    .iter()
                    .map(|v| symbol(&v.name.name, 22, v.name.span, v.name.span, vec![]))
                    .collect();
                out.push(symbol(&e.name.name, 10, e.span, e.name.span, children));
            }
            Item::Trait(t) => {
                let children = t
                    .methods
                    .iter()
                    .map(|m| symbol(&m.name.name, 6, m.span, m.name.span, vec![]))
                    .collect();
                out.push(symbol(&t.name.name, 11, t.span, t.name.span, children));
            }
            Item::Impl(i) => {
                let target = match &i.target.kind {
                    TypeExprKind::Named { name, .. } => name.name.clone(),
                    _ => "record".into(),
                };
                let name = match &i.trait_name {
                    Some(t) => format!("impl {} for {target}", t.name),
                    None => format!("impl {target}"),
                };
                let children = i
                    .methods
                    .iter()
                    .map(|m| symbol(&m.name.name, 6, m.span, m.name.span, vec![]))
                    .collect();
                out.push(symbol(&name, 5, i.span, i.target.span, children));
            }
            Item::Use(_) => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uris_and_paths() {
        let p = uri_to_path("file:///d%3A/Apps/My%20Project/main.elo");
        assert_eq!(p, PathBuf::from("d:/Apps/My Project/main.elo"));
        assert_eq!(
            uri_to_path("file:///home/u/a.elo"),
            PathBuf::from("/home/u/a.elo")
        );
        assert_eq!(
            path_to_uri(Path::new("/home/u/my file.elo")),
            "file:///home/u/my%20file.elo"
        );
    }

    #[test]
    fn utf16_offsets() {
        let text = "let a = 1\nlet اسم = \"😀x\"\n";
        assert_eq!(offset_at(text, 0, 4), 4);
        let line2 = text.find("let اسم").unwrap();
        assert_eq!(offset_at(text, 1, 4), line2 + 4);
        // The emoji is two UTF-16 units.
        let x = text.find('x').unwrap();
        assert_eq!(offset_at(text, 1, 13), x);
        assert_eq!(offset_at(text, 9, 0), text.len());
    }

    #[test]
    fn outline() {
        let symbols = document_symbols(
            "type S = { a: Int }\nenum E {\n  A\n  B\n}\nfn main() {}\nimpl E {\n  fn f(self) {}\n}\n",
        );
        let names: Vec<&str> = symbols
            .iter()
            .map(|s| s["name"].as_str().unwrap())
            .collect();
        assert_eq!(names, ["S", "E", "main", "impl E"]);
        assert_eq!(symbols[1]["children"].as_array().unwrap().len(), 2);
    }
}
