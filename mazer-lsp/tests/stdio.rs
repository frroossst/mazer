//! End to end: spawn the built server and speak JSON-RPC over its stdio,
//! with the `Content-Length` framing written by hand.

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

const TIMEOUT: Duration = Duration::from_secs(20);
const URI: &str = "file:///tmp/mazer-lsp-test/note.zr";

struct Client {
    child: Child,
    stdin: Option<ChildStdin>,
    incoming: Receiver<Value>,
    /// Notifications that arrived while waiting for something else.
    pending: VecDeque<Value>,
    next_id: i64,
}

impl Client {
    fn spawn() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_mazer-lsp"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("the server starts");
        let stdin = child.stdin.take();
        let stdout = child.stdout.take().expect("stdout");
        let (tx, incoming) = mpsc::channel();
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            while let Some(message) = read_frame(&mut reader) {
                if tx.send(message).is_err() {
                    return;
                }
            }
        });
        Self {
            child,
            stdin,
            incoming,
            pending: VecDeque::new(),
            next_id: 0,
        }
    }

    fn send(&mut self, message: &Value) {
        let body = message.to_string();
        let stdin = self.stdin.as_mut().expect("stdin open");
        write!(stdin, "Content-Length: {}\r\n\r\n{body}", body.len()).unwrap();
        stdin.flush().unwrap();
    }

    fn notify(&mut self, method: &str, params: Value) {
        let mut message = json!({"jsonrpc": "2.0", "method": method});
        message["params"] = params;
        self.send(&message);
    }

    /// Sends a request and waits for its response (the whole response
    /// object, with `result` or `error`).
    fn request(&mut self, method: &str, params: Value) -> Value {
        self.next_id += 1;
        let id = self.next_id;
        let mut message = json!({"jsonrpc": "2.0", "id": id, "method": method});
        message["params"] = params;
        self.send(&message);
        let deadline = Instant::now() + TIMEOUT;
        loop {
            let message = self.next_message(deadline, method);
            if message.get("id") == Some(&json!(id)) && message.get("method").is_none() {
                return message;
            }
            self.pending.push_back(message);
        }
    }

    /// The result of a request that must succeed.
    fn ok(&mut self, method: &str, params: Value) -> Value {
        let response = self.request(method, params);
        assert!(response.get("error").is_none(), "{method} failed: {response}");
        response["result"].clone()
    }

    /// Waits for a notification with this method.
    fn notification(&mut self, method: &str) -> Value {
        if let Some(i) = self.pending.iter().position(|m| m["method"] == method) {
            return self.pending.remove(i).unwrap();
        }
        let deadline = Instant::now() + TIMEOUT;
        loop {
            let message = self.next_message(deadline, method);
            if message["method"] == method {
                return message;
            }
            self.pending.push_back(message);
        }
    }

    fn next_message(&self, deadline: Instant, waiting_for: &str) -> Value {
        let left = deadline.saturating_duration_since(Instant::now());
        self.incoming
            .recv_timeout(left)
            .unwrap_or_else(|_| panic!("timed out waiting for {waiting_for}"))
    }

    /// Waits for the process to end.
    fn wait(&mut self) -> std::process::ExitStatus {
        drop(self.stdin.take());
        let deadline = Instant::now() + TIMEOUT;
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                return status;
            }
            assert!(Instant::now() < deadline, "the server did not exit");
            thread::sleep(Duration::from_millis(20));
        }
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

fn read_frame(reader: &mut impl BufRead) -> Option<Value> {
    let mut length = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).ok()? == 0 {
            return None;
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some(value) = line.strip_prefix("Content-Length: ") {
            length = value.parse::<usize>().ok();
        }
    }
    let mut body = vec![0; length?];
    reader.read_exact(&mut body).ok()?;
    serde_json::from_slice(&body).ok()
}

fn position(line: u32, character: u32) -> Value {
    json!({"textDocument": {"uri": URI}, "position": {"line": line, "character": character}})
}

/// Line and UTF-16 column of the first occurrence of `needle` in `text`
/// (plus `offset` characters).
fn find(text: &str, needle: &str, offset: usize) -> (u32, u32) {
    let at = text.find(needle).expect("needle in text");
    let before = &text[..at];
    let line = before.matches('\n').count();
    let col = before.rsplit('\n').next().unwrap().encode_utf16().count() + offset;
    (u32::try_from(line).unwrap(), u32::try_from(col).unwrap())
}

fn initialize(client: &mut Client) -> Value {
    let result = client.ok(
        "initialize",
        json!({
            "processId": null,
            "rootUri": null,
            "capabilities": {"textDocument": {"completion": {"completionItem": {"snippetSupport": true}}}},
        }),
    );
    client.notify("initialized", json!({}));
    result
}

fn http_get(url: &str) -> String {
    let rest = url.strip_prefix("http://").expect("an http URL");
    let (host, path) = rest.split_once('/').expect("a path");
    let mut stream = TcpStream::connect(host).expect("the preview server answers");
    stream.set_read_timeout(Some(TIMEOUT)).unwrap();
    write!(stream, "GET /{path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n").unwrap();
    let mut out = String::new();
    stream.read_to_string(&mut out).unwrap();
    out
}

/// The capabilities `initialize` advertises.
fn check_capabilities(init: &Value) {
    let caps = &init["capabilities"];
    assert_eq!(caps["positionEncoding"], "utf-16");
    assert_eq!(caps["textDocumentSync"]["change"], 1);
    assert!(caps["semanticTokensProvider"]["legend"]["tokenTypes"].as_array().is_some_and(|t| !t.is_empty()));
    assert_eq!(init["serverInfo"]["name"], "mazer-lsp");
    // Space is not a completion trigger (it is one for signature help).
    assert_eq!(caps["completionProvider"]["triggerCharacters"], json!(["(", ":", "'"]));
    assert_eq!(caps["completionProvider"]["resolveProvider"], true);
    assert_eq!(caps["signatureHelpProvider"]["triggerCharacters"], json!([" ", "("]));
}

/// Completion items at the head of `(integral` resolve to full docs; the
/// `let` variable and the integration variable of `text`.
fn completion_and_variables(client: &mut Client, text: &str, items: &Value) {
    let items = items.as_array().expect("an array of items");
    let integral = items.iter().find(|i| i["label"] == "integral").expect("integral");
    assert!(
        integral["documentation"].is_string(),
        "the list carries a one-line doc only: {integral}"
    );
    // The full docs come on resolve.
    let resolved = client.ok("completionItem/resolve", integral.clone());
    let doc = resolved["documentation"]["value"].as_str().unwrap();
    assert!(doc.contains("**Parameters**"), "{doc}");

    // Hover on the `let` variable inside the integral: its value. The
    // integration variable is not let-bound, so it stays a plain name.
    let (y_line, y_col) = find(text, "(sin y)", 5);
    let hover = client.ok("textDocument/hover", position(y_line, y_col));
    let value = hover["contents"]["value"].as_str().unwrap();
    assert!(value.contains("`let` variable `y`, shown as:"), "{value}");
    let (x_line, x_col) = find(text, "(sin y) x", 8);
    let def = client.ok("textDocument/definition", position(x_line, x_col));
    assert_eq!(def, Value::Null, "the integration variable is not bound");
}

/// Garbage: the server answers with errors and stays alive.
fn garbage_is_answered(client: &mut Client) {
    let unknown = client.request("mazer/nope", json!({}));
    assert_eq!(unknown["error"]["code"], -32601, "{unknown}");
    let wrong = client.request("textDocument/hover", json!({"what": 1}));
    assert_eq!(wrong["error"]["code"], -32602, "{wrong}");
    let wrong = client.request("textDocument/completion", json!([1, 2, 3]));
    assert_eq!(wrong["error"]["code"], -32602, "{wrong}");
    client.notify("$/somethingUnknown", json!({"x": 1}));
    client.notify("textDocument/didChange", json!({"bogus": true}));
    let far = client.request("textDocument/hover", position(99_999, 99_999));
    assert!(far.get("error").is_none(), "{far}");
    let far = client.request("textDocument/completion", position(u32::MAX, u32::MAX));
    assert!(far.get("error").is_none(), "{far}");
    let other = client.request(
        "textDocument/hover",
        json!({"textDocument": {"uri": "file:///not/open.zr"}, "position": {"line": 0, "character": 0}}),
    );
    assert_eq!(other["result"], Value::Null);
    let bad_command = client.request("workspace/executeCommand", json!({"command": "mazer.preview", "arguments": [42]}));
    assert!(bad_command.get("error").is_some());
}

#[test]
fn a_whole_session() {
    let mut client = Client::spawn();
    let init = initialize(&mut client);
    check_capabilities(&init);

    // A note with an error.
    let text = "# τ notes 😭\n\n(show (frac 1))\n\n(let y 2)\n\n(show (tag e) (integral (sin y) x))\n\nSee (ref e).\n";
    client.notify(
        "textDocument/didOpen",
        json!({"textDocument": {"uri": URI, "languageId": "zr", "version": 1, "text": text}}),
    );
    let diags = client.notification("textDocument/publishDiagnostics");
    let list = diags["params"]["diagnostics"].as_array().unwrap();
    assert_eq!(list.len(), 1, "{diags}");
    assert_eq!(list[0]["code"], "arity");
    assert_eq!(list[0]["severity"], 1);
    assert_eq!(list[0]["source"], "mazer");
    assert_eq!(list[0]["range"]["start"], json!({"line": 2, "character": 6}));

    // Completion at the head of `(integral`.
    let (line, col) = find(text, "(integral", 4);
    let items = client.ok("textDocument/completion", position(line, col));
    completion_and_variables(&mut client, text, &items);

    // Hover on it.
    let hover = client.ok("textDocument/hover", position(line, col));
    let value = hover["contents"]["value"].as_str().unwrap();
    assert!(value.contains("(integral body var [lower] [upper])"), "{value}");

    // Signature help on its second argument.
    let (line, col) = find(text, "(sin y) x)", 8);
    let help = client.ok("textDocument/signatureHelp", position(line, col));
    assert_eq!(help["signatures"][0]["label"], "(integral body var [lower] [upper])");
    assert_eq!(help["activeParameter"], 1);

    // Definition of `(ref e)`: the tag.
    let (line, col) = find(text, "ref e", 4);
    let def = client.ok("textDocument/definition", position(line, col));
    assert_eq!(def["uri"], URI);
    assert_eq!(def["range"]["start"]["line"], 6);

    // Semantic tokens.
    let tokens = client.ok("textDocument/semanticTokens/full", json!({"textDocument": {"uri": URI}}));
    let data = tokens["data"].as_array().unwrap();
    assert!(!data.is_empty() && data.len() % 5 == 0);

    // Document symbols: the heading with its definitions.
    let symbols = client.ok("textDocument/documentSymbol", json!({"textDocument": {"uri": URI}}));
    assert_eq!(symbols[0]["name"], "τ notes 😭");

    // Fix the error: the diagnostics clear.
    let fixed = text.replace("(frac 1)", "(frac 1 2)");
    client.notify(
        "textDocument/didChange",
        json!({"textDocument": {"uri": URI, "version": 2}, "contentChanges": [{"text": fixed}]}),
    );
    let diags = client.notification("textDocument/publishDiagnostics");
    assert_eq!(diags["params"]["version"], 2);
    assert_eq!(diags["params"]["diagnostics"], json!([]));

    // Live preview.
    let url = client.ok("workspace/executeCommand", json!({"command": "mazer.preview", "arguments": [URI]}));
    let url = url.as_str().expect("a URL").to_owned();
    assert!(url.starts_with("http://127.0.0.1:"), "{url}");
    let shown = client.notification("window/showMessage");
    assert!(shown["params"]["message"].as_str().unwrap().contains(&url));
    let page = http_get(&url);
    assert!(page.starts_with("HTTP/1.1 200 OK"), "{page}");
    assert!(page.contains("<math display=\"block\">"), "the rendered math is served");
    assert!(page.contains("∫"));
    assert!(page.contains("/version"), "with the reload script");
    let version = http_get(&format!("{url}/version"));
    assert!(version.ends_with("\r\n\r\n1"), "{version}");
    // A change updates what the preview serves.
    let changed = fixed.replace("(sin y)", "(cos y)");
    client.notify(
        "textDocument/didChange",
        json!({"textDocument": {"uri": URI, "version": 3}, "contentChanges": [{"text": changed}]}),
    );
    let _ = client.notification("textDocument/publishDiagnostics");
    assert!(http_get(&format!("{url}/version")).ends_with("\r\n\r\n2"));
    assert!(http_get(&url).contains("cos"));

    garbage_is_answered(&mut client);
    // Still answering properly.
    let (line, col) = find(&changed, "(cos", 2);
    let hover = client.ok("textDocument/hover", position(line, col));
    assert!(hover["contents"]["value"].as_str().unwrap().contains("cos"));

    // The preview refuses a request with two Host headers.
    let host = url.strip_prefix("http://").unwrap().split('/').next().unwrap().to_owned();
    let mut stream = TcpStream::connect(&host).unwrap();
    stream.set_read_timeout(Some(TIMEOUT)).unwrap();
    write!(stream, "GET /doc/1 HTTP/1.1\r\nHost: {host}\r\nHost: evil.example\r\n\r\n").unwrap();
    let mut answer = String::new();
    stream.read_to_string(&mut answer).unwrap();
    assert!(answer.starts_with("HTTP/1.1 400"), "{answer}");

    // Close: diagnostics are cleared, and the preview stops serving it.
    client.notify("textDocument/didClose", json!({"textDocument": {"uri": URI}}));
    let diags = client.notification("textDocument/publishDiagnostics");
    assert_eq!(diags["params"]["diagnostics"], json!([]));
    assert!(http_get(&url).starts_with("HTTP/1.1 404"), "a closed note is not served");

    // Shutdown and exit cleanly.
    let shutdown = client.request("shutdown", Value::Null);
    assert_eq!(shutdown["result"], Value::Null);
    assert!(shutdown.get("error").is_none());
    client.notify("exit", Value::Null);
    let status = client.wait();
    assert!(status.success(), "{status}");
}

#[test]
fn an_absurd_content_length_ends_the_session_cleanly() {
    let mut client = Client::spawn();
    initialize(&mut client);
    // Far larger than any message: refused before allocating, not an abort.
    let stdin = client.stdin.as_mut().unwrap();
    stdin.write_all(b"Content-Length: 99999999999999\r\n\r\n{}").unwrap();
    stdin.flush().unwrap();
    let status = client.wait();
    assert_eq!(status.code(), Some(1), "{status}");
}

#[test]
fn malformed_framing_ends_the_session() {
    let mut client = Client::spawn();
    initialize(&mut client);
    let stdin = client.stdin.as_mut().unwrap();
    stdin.write_all(b"Content-Length: 2\n\n{}").unwrap();
    stdin.flush().unwrap();
    let status = client.wait();
    assert_eq!(status.code(), Some(1), "{status}");
}

#[test]
fn exit_without_shutdown_is_an_error_status() {
    let mut client = Client::spawn();
    initialize(&mut client);
    client.notify("exit", Value::Null);
    let status = client.wait();
    assert_eq!(status.code(), Some(1));
}

#[test]
fn the_client_going_away_ends_the_server() {
    let mut client = Client::spawn();
    initialize(&mut client);
    let status = client.wait(); // closes stdin
    assert_eq!(status.code(), Some(1));
}
