//! Live preview: a tiny HTTP server on `127.0.0.1` (an OS-assigned port),
//! started on the first `mazer.preview` command, built on `std::net` only.
//!
//! - `GET /doc/<id>` is the note rendered as a full page, with a few lines
//!   of JavaScript that poll `GET /doc/<id>/version` and reload the page when
//!   the note changes (keeping the scroll position).
//! - `GET /` lists the notes being previewed.
//! - Anything else is a 404; a malformed request a 400; a request whose
//!   `Host` is not this loopback address a 403 (so a web page cannot reach
//!   the notes through DNS rebinding).
//!
//! The accept loop runs on its own thread; each connection is answered on a
//! short-lived thread and closed after one response. Limits keep a
//! misbehaving client from holding the server:
//! - the whole request head must arrive within [`Limits::head_deadline`]
//!   (5 s) of the connection, however slowly it trickles in; otherwise the
//!   connection is closed without an answer;
//! - at most [`Limits::max_connections`] (32) connections are served at
//!   once; one more is closed at once;
//! - a request with more than one `Host` header is a 400, one whose `Host`
//!   is not this loopback address a 403;
//! - a note that is closed in the editor is no longer served (404).
//!
//! The listener is bound to `127.0.0.1` only. [`Preview::stop`] ends the
//! accept loop and joins it.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::io::{self, Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// Longest request head read before giving up with a 400.
const MAX_REQUEST: usize = 8 * 1024;

/// How much the preview server puts up with.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// The whole request head must have arrived this long after the
    /// connection was accepted.
    pub head_deadline: Duration,
    /// Connections served at the same time; more are closed at once.
    pub max_connections: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            head_deadline: Duration::from_secs(5),
            max_connections: 32,
        }
    }
}

/// One previewed note.
struct Page {
    title: String,
    html: String,
    version: u64,
}

#[derive(Default)]
struct Shared {
    pages: Mutex<HashMap<u32, Page>>,
    stop: AtomicBool,
    /// Connections being served now.
    active: AtomicUsize,
}

/// Counts one connection as active while it lives.
struct Slot(Arc<Shared>);

impl Drop for Slot {
    fn drop(&mut self) {
        self.0.active.fetch_sub(1, Ordering::SeqCst);
    }
}

impl Shared {
    fn pages(&self) -> MutexGuard<'_, HashMap<u32, Page>> {
        self.pages.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The running preview server.
pub struct Preview {
    addr: SocketAddr,
    shared: Arc<Shared>,
    thread: Option<JoinHandle<()>>,
}

impl Preview {
    /// Binds a loopback port and starts serving.
    ///
    /// # Errors
    /// If no port can be bound or the thread cannot be started.
    pub fn start() -> io::Result<Self> {
        Self::start_with(Limits::default())
    }

    /// [`Self::start`] with other limits (tests use short ones).
    ///
    /// # Errors
    /// If no port can be bound or the thread cannot be started.
    pub fn start_with(limits: Limits) -> io::Result<Self> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
        let addr = listener.local_addr()?;
        let shared = Arc::new(Shared::default());
        let state = Arc::clone(&shared);
        let thread = thread::Builder::new()
            .name("mazer-preview".to_owned())
            .spawn(move || accept_loop(&listener, &state, addr, limits))?;
        Ok(Self {
            addr,
            shared,
            thread: Some(thread),
        })
    }

    /// The URL of the page for note `id`.
    #[must_use]
    pub fn url(&self, id: u32) -> String {
        format!("http://{}/doc/{id}", self.addr)
    }

    /// Sets what note `id` shows; open pages reload.
    pub fn update(&self, id: u32, title: &str, html: String) {
        // Only the server thread writes, so reading the version first is safe.
        let version = self.shared.pages().get(&id).map_or(1, |p| p.version + 1);
        self.shared.pages().insert(
            id,
            Page {
                title: title.to_owned(),
                html,
                version,
            },
        );
    }

    /// Stops serving note `id` (it was closed in the editor): its page and
    /// version are 404 from now on.
    pub fn remove(&self, id: u32) {
        self.shared.pages().remove(&id);
    }

    /// Stops serving and joins the accept thread.
    pub fn stop(mut self) {
        self.shutdown();
    }

    fn shutdown(&mut self) {
        self.shared.stop.store(true, Ordering::SeqCst);
        let Some(thread) = self.thread.take() else {
            return;
        };
        // Wake the blocking `accept`; join only if that worked.
        if TcpStream::connect_timeout(&self.addr, Duration::from_secs(1)).is_ok() {
            let _ = thread.join();
        }
    }
}

impl Drop for Preview {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn accept_loop(listener: &TcpListener, shared: &Arc<Shared>, addr: SocketAddr, limits: Limits) {
    for stream in listener.incoming() {
        if shared.stop.load(Ordering::SeqCst) {
            return;
        }
        let Ok(stream) = stream else {
            continue;
        };
        if shared.active.fetch_add(1, Ordering::SeqCst) >= limits.max_connections {
            shared.active.fetch_sub(1, Ordering::SeqCst);
            drop(stream); // too many at once: close it now
            continue;
        }
        let slot = Slot(Arc::clone(shared));
        let _ = thread::Builder::new()
            .name("mazer-preview-conn".to_owned())
            .spawn(move || {
                let _ = serve(stream, &slot.0, addr, limits.head_deadline);
                drop(slot);
            });
    }
}

/// A response: status line, content type, body.
struct Response {
    status: &'static str,
    content_type: &'static str,
    body: String,
}

impl Response {
    const fn html(body: String) -> Self {
        Self {
            status: "200 OK",
            content_type: "text/html; charset=utf-8",
            body,
        }
    }

    fn text(status: &'static str, body: &str) -> Self {
        Self {
            status,
            content_type: "text/plain; charset=utf-8",
            body: body.to_owned(),
        }
    }
}

fn serve(mut stream: TcpStream, shared: &Shared, addr: SocketAddr, deadline: Duration) -> io::Result<()> {
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    let (response, head_only) = match read_head(&mut stream, Instant::now() + deadline)? {
        Head::TooSlow => return Ok(()), // close without an answer
        Head::Bad => (Response::text("400 Bad Request", "bad request\n"), false),
        Head::Request(head) => (respond(&head, shared, addr), head.starts_with("HEAD ")),
    };
    let mut out = format!(
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        response.status,
        response.content_type,
        response.body.len()
    )
    .into_bytes();
    if !head_only {
        out.extend_from_slice(response.body.as_bytes());
    }
    stream.write_all(&out)?;
    stream.flush()
}

/// What came in as a request head.
enum Head {
    /// The head, up to the blank line.
    Request(String),
    /// Too long, not UTF-8, or the connection ended first.
    Bad,
    /// The deadline passed first.
    TooSlow,
}

/// Reads the request head (up to the blank line) before `deadline`.
fn read_head(stream: &mut TcpStream, deadline: Instant) -> io::Result<Head> {
    let mut buf = Vec::with_capacity(1024);
    let mut chunk = [0u8; 1024];
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Ok(Head::TooSlow);
        }
        stream.set_read_timeout(Some(left))?;
        let n = match stream.read(&mut chunk) {
            Ok(n) => n,
            Err(e) if matches!(e.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut) => return Ok(Head::TooSlow),
            Err(e) => return Err(e),
        };
        if n == 0 {
            return Ok(Head::Bad);
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(end) = find(&buf, b"\r\n\r\n").or_else(|| find(&buf, b"\n\n")) {
            buf.truncate(end);
            return Ok(String::from_utf8(buf).map_or(Head::Bad, Head::Request));
        }
        if buf.len() > MAX_REQUEST {
            return Ok(Head::Bad);
        }
    }
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// Answers one parsed request head.
fn respond(head: &str, shared: &Shared, addr: SocketAddr) -> Response {
    let mut lines = head.lines();
    let mut parts = lines.next().unwrap_or("").split_whitespace();
    let (Some(method), Some(target), Some(version)) = (parts.next(), parts.next(), parts.next()) else {
        return Response::text("400 Bad Request", "bad request\n");
    };
    if !version.starts_with("HTTP/") || parts.next().is_some() {
        return Response::text("400 Bad Request", "bad request\n");
    }
    if method != "GET" && method != "HEAD" {
        return Response::text("405 Method Not Allowed", "only GET\n");
    }
    let hosts: Vec<&str> = lines
        .filter_map(|l| l.split_once(':'))
        .filter(|(name, _)| name.trim().eq_ignore_ascii_case("host"))
        .map(|(_, value)| value.trim())
        .collect();
    if hosts.len() > 1 {
        return Response::text("400 Bad Request", "more than one Host header\n");
    }
    if let Some(host) = hosts.first()
        && *host != format!("127.0.0.1:{}", addr.port())
        && *host != format!("localhost:{}", addr.port())
    {
        return Response::text("403 Forbidden", "wrong host\n");
    }
    let path = target.split(['?', '#']).next().unwrap_or("");
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    let missing = || Response::text("404 Not Found", "no such note\n");
    match segments.as_slice() {
        [] => Response::html(index(shared)),
        ["doc", id] => id
            .parse()
            .ok()
            .and_then(|id: u32| shared.pages().get(&id).map(|p| with_reload(&p.html, id, p.version)))
            .map_or_else(missing, Response::html),
        ["doc", id, "version"] => id
            .parse()
            .ok()
            .and_then(|id: u32| shared.pages().get(&id).map(|p| p.version))
            .map_or_else(missing, |v| Response::text("200 OK", &v.to_string())),
        _ => Response::text("404 Not Found", "not found\n"),
    }
}

/// The list of previewed notes.
fn index(shared: &Shared) -> String {
    let mut notes: Vec<(u32, String)> = shared.pages().iter().map(|(id, p)| (*id, escape(&p.title))).collect();
    notes.sort_unstable();
    let mut body = String::from(
        "<!DOCTYPE html><html><head><meta charset=\"UTF-8\"><title>mazer preview</title></head><body><h1>mazer preview</h1><ul>",
    );
    for (id, title) in notes {
        let _ = write!(body, "<li><a href=\"/doc/{id}\">{title}</a></li>");
    }
    body.push_str("</ul></body></html>");
    body
}

/// The page with the reload script inserted before `</body>` (or appended).
fn with_reload(html: &str, id: u32, version: u64) -> String {
    let script = format!(
        "<script>(function(){{\
var v='{version}',k='mazer-scroll-{id}';\
try{{var y=sessionStorage.getItem(k);if(y!==null){{sessionStorage.removeItem(k);addEventListener('load',function(){{scrollTo(0,+y)}});}}}}catch(e){{}}\
function poll(){{fetch('/doc/{id}/version',{{cache:'no-store'}}).then(function(r){{return r.ok?r.text():v}}).then(function(t){{\
if(t!==v){{try{{sessionStorage.setItem(k,String(scrollY))}}catch(e){{}}location.reload();}}else setTimeout(poll,300);}},function(){{setTimeout(poll,2000)}});}}\
setTimeout(poll,300);}})();</script>"
    );
    html.rfind("</body>").map_or_else(
        || format!("{html}{script}"),
        |at| format!("{}{script}{}", &html[..at], &html[at..]),
    )
}

fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get(addr: SocketAddr, request: &[u8]) -> String {
        let mut stream = TcpStream::connect(addr).unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        // A refused connection may already be closed.
        let _ = stream.write_all(request);
        let mut out = String::new();
        let _ = stream.read_to_string(&mut out);
        out
    }

    #[test]
    fn serves_pages_and_versions() {
        let preview = Preview::start().unwrap();
        let addr = preview.addr;
        preview.update(1, "note", "<html><body><math>x</math></body></html>".to_owned());
        let host = format!("Host: 127.0.0.1:{}\r\n", addr.port());
        let page = get(addr, format!("GET /doc/1 HTTP/1.1\r\n{host}\r\n").as_bytes());
        assert!(page.starts_with("HTTP/1.1 200 OK"), "{page}");
        assert!(page.contains("<math>x</math><script>"), "{page}");
        assert!(page.contains("var v='1'"));
        let version = get(addr, b"GET /doc/1/version HTTP/1.1\r\n\r\n");
        assert!(version.ends_with("\r\n\r\n1"), "{version}");
        preview.update(1, "note", "<html><body>y</body></html>".to_owned());
        assert!(get(addr, b"GET /doc/1/version HTTP/1.1\r\n\r\n").ends_with("\r\n\r\n2"));
        let index = get(addr, b"GET / HTTP/1.1\r\n\r\n");
        assert!(index.contains("href=\"/doc/1\""));
        preview.stop();
    }

    #[test]
    fn rejects_bad_requests() {
        let preview = Preview::start().unwrap();
        let addr = preview.addr;
        assert!(get(addr, b"GET /doc/9 HTTP/1.1\r\n\r\n").starts_with("HTTP/1.1 404"));
        assert!(get(addr, b"GET /../etc/passwd HTTP/1.1\r\n\r\n").starts_with("HTTP/1.1 404"));
        assert!(get(addr, b"POST /doc/1 HTTP/1.1\r\n\r\n").starts_with("HTTP/1.1 405"));
        assert!(get(addr, b"garbage\r\n\r\n").starts_with("HTTP/1.1 400"));
        assert!(get(addr, b"\xff\xfe\r\n\r\n").starts_with("HTTP/1.1 400"));
        assert!(get(addr, &[b'a'; MAX_REQUEST + 10]).starts_with("HTTP/1.1 400"));
        assert!(get(addr, b"GET / HTTP/1.1\r\nHost: evil.example\r\n\r\n").starts_with("HTTP/1.1 403"));
        // A connection that sends nothing and closes.
        drop(TcpStream::connect(addr).unwrap());
        // Still serving.
        assert!(get(addr, b"GET / HTTP/1.1\r\n\r\n").starts_with("HTTP/1.1 200"));
        preview.stop();
    }

    #[test]
    fn host_headers() {
        let preview = Preview::start().unwrap();
        let addr = preview.addr;
        let port = addr.port();
        let ok = format!("GET / HTTP/1.1\r\nHost: localhost:{port}\r\n\r\n");
        assert!(get(addr, ok.as_bytes()).starts_with("HTTP/1.1 200"));
        let two = format!("GET / HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nhost: evil.example\r\n\r\n");
        assert!(get(addr, two.as_bytes()).starts_with("HTTP/1.1 400"), "two Host headers");
        let same_twice = format!("GET / HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nHost: 127.0.0.1:{port}\r\n\r\n");
        assert!(get(addr, same_twice.as_bytes()).starts_with("HTTP/1.1 400"));
        for host in ["evil.example", "127.0.0.1", "127.0.0.1:1", "192.168.0.1:80", "[::1]:1"] {
            let request = format!("GET / HTTP/1.1\r\nHost: {host}\r\n\r\n");
            assert!(get(addr, request.as_bytes()).starts_with("HTTP/1.1 403"), "{host}");
        }
        assert!(addr.ip().is_loopback());
        preview.stop();
    }

    #[test]
    fn a_closed_note_is_no_longer_served() {
        let preview = Preview::start().unwrap();
        let addr = preview.addr;
        preview.update(3, "note", "<html><body>x</body></html>".to_owned());
        assert!(get(addr, b"GET /doc/3 HTTP/1.1\r\n\r\n").starts_with("HTTP/1.1 200"));
        preview.remove(3);
        assert!(get(addr, b"GET /doc/3 HTTP/1.1\r\n\r\n").starts_with("HTTP/1.1 404"));
        assert!(get(addr, b"GET /doc/3/version HTTP/1.1\r\n\r\n").starts_with("HTTP/1.1 404"));
        assert!(!get(addr, b"GET / HTTP/1.1\r\n\r\n").contains("/doc/3"));
        preview.stop();
    }

    #[test]
    fn a_slow_request_head_is_cut_off() {
        let limits = Limits {
            head_deadline: Duration::from_millis(400),
            max_connections: 32,
        };
        let preview = Preview::start_with(limits).unwrap();
        let addr = preview.addr;
        let mut stream = TcpStream::connect(addr).unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        // A byte every 100 ms never finishes the head: each read is quick,
        // but the total deadline still ends it.
        let start = Instant::now();
        let mut closed = false;
        for byte in b"GET / HTTP/1.1\r\nX-Slow: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" {
            if stream.write_all(&[*byte]).is_err() {
                closed = true;
                break;
            }
            thread::sleep(Duration::from_millis(100));
            if start.elapsed() > Duration::from_secs(2) {
                break;
            }
        }
        let mut out = Vec::new();
        let _ = stream.read_to_end(&mut out);
        assert!(closed || out.is_empty(), "no answer to a request that never ended: {out:?}");
        assert!(start.elapsed() < Duration::from_secs(4), "{:?}", start.elapsed());
        // A prompt request is still answered.
        assert!(get(addr, b"GET / HTTP/1.1\r\n\r\n").starts_with("HTTP/1.1 200"));
        preview.stop();
    }

    #[test]
    fn too_many_connections_are_closed_at_once() {
        let limits = Limits {
            head_deadline: Duration::from_secs(10),
            max_connections: 4,
        };
        let preview = Preview::start_with(limits).unwrap();
        let addr = preview.addr;
        // Four idle connections hold every slot.
        let idle: Vec<TcpStream> = (0..4).map(|_| TcpStream::connect(addr).unwrap()).collect();
        let deadline = Instant::now() + Duration::from_secs(5);
        while preview.shared.active.load(Ordering::SeqCst) < 4 {
            assert!(Instant::now() < deadline, "the idle connections were not accepted");
            thread::sleep(Duration::from_millis(10));
        }
        // The fifth is closed without an answer, right away.
        let start = Instant::now();
        let refused = get(addr, b"GET / HTTP/1.1\r\n\r\n");
        assert_eq!(refused, "", "{refused}");
        assert!(start.elapsed() < Duration::from_secs(2));
        // Once they go away, requests are served again.
        drop(idle);
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if get(addr, b"GET / HTTP/1.1\r\n\r\n").starts_with("HTTP/1.1 200") {
                break;
            }
            assert!(Instant::now() < deadline, "not serving again");
            thread::sleep(Duration::from_millis(20));
        }
        preview.stop();
    }
}
