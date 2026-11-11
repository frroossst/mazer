//! The stdio transport: `Content-Length` framing over stdin and stdout.
//!
//! `lsp-server`'s own stdio transport allocates whatever `Content-Length` a
//! message announces before reading it, so a header such as
//! `Content-Length: 99999999999999` aborts the process with an allocation
//! failure. This transport reads the framing itself, with limits, and hands
//! each message body to `lsp_server::Message::read`, so messages are parsed
//! exactly as the library parses them. The server loop talks to it through
//! an in-memory `lsp_server::Connection`.
//!
//! Limits and failures (each ends the session, as malformed framing does in
//! `lsp-server`; the binary then exits with status 1 and the reason on
//! stderr):
//! - a header line longer than [`MAX_HEADER_LINE`] bytes, or one that does
//!   not end in `\r\n` or is not `Name: value`;
//! - no `Content-Length`, or one that is not a number;
//! - a body larger than [`MAX_MESSAGE`] bytes (64 MiB; a note is a few
//!   hundred kilobytes);
//! - a body that is not a JSON-RPC message.

use std::io::{self, BufRead, Cursor, Read, Write};
use std::thread::{self, JoinHandle};

use lsp_server::{Connection, Message};

/// The largest message body accepted.
pub const MAX_MESSAGE: usize = 64 * 1024 * 1024;

/// The longest header line accepted.
pub const MAX_HEADER_LINE: u64 = 8 * 1024;

/// The threads moving messages between stdio and the server's connection.
pub struct IoThreads {
    reader: JoinHandle<io::Result<()>>,
    writer: JoinHandle<io::Result<()>>,
}

impl IoThreads {
    /// Waits for both threads; the first failure is returned.
    ///
    /// # Errors
    /// What ended the reader (malformed or oversized input) or the writer.
    pub fn join(self) -> io::Result<()> {
        let reader = self.reader.join().unwrap_or_else(|_| Err(io::Error::other("the reader thread panicked")));
        let writer = self.writer.join().unwrap_or_else(|_| Err(io::Error::other("the writer thread panicked")));
        reader.and(writer)
    }
}

/// A connection for the server over stdin and stdout.
///
/// # Errors
/// If a thread cannot be started.
pub fn stdio() -> io::Result<(Connection, IoThreads)> {
    let (server, client) = Connection::memory();
    let Connection {
        sender: to_server,
        receiver: from_server,
    } = client;
    let reader = thread::Builder::new().name("mazer-lsp-reader".to_owned()).spawn(move || {
        let stdin = io::stdin();
        let mut input = stdin.lock();
        while let Some(message) = read_message(&mut input)? {
            let exit = matches!(&message, Message::Notification(n) if n.method == "exit");
            if to_server.send(message).is_err() || exit {
                break;
            }
        }
        Ok(())
    })?;
    let writer = thread::Builder::new().name("mazer-lsp-writer".to_owned()).spawn(move || {
        let stdout = io::stdout();
        let mut out = stdout.lock();
        for message in from_server {
            message.write(&mut out)?;
            out.flush()?;
        }
        Ok(())
    })?;
    Ok((server, IoThreads { reader, writer }))
}

fn invalid(message: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

/// Reads one framed message; `None` at the end of the input.
///
/// # Errors
/// Malformed or oversized framing, a body that is not a message, or an I/O
/// error.
pub fn read_message(input: &mut impl BufRead) -> io::Result<Option<Message>> {
    let mut length: Option<usize> = None;
    let mut line = Vec::new();
    loop {
        line.clear();
        let n = input.by_ref().take(MAX_HEADER_LINE).read_until(b'\n', &mut line)?;
        if n == 0 {
            return Ok(None);
        }
        let Some(header) = line.strip_suffix(b"\r\n") else {
            return Err(invalid(format!(
                "malformed header (not ended by \\r\\n within {MAX_HEADER_LINE} bytes)"
            )));
        };
        if header.is_empty() {
            break;
        }
        let header = std::str::from_utf8(header).map_err(|_| invalid("a header is not UTF-8".to_owned()))?;
        let Some((name, value)) = header.split_once(": ") else {
            return Err(invalid(format!("malformed header: {header:?}")));
        };
        if name.eq_ignore_ascii_case("Content-Length") {
            let size: u64 = value.trim().parse().map_err(|_| invalid(format!("bad Content-Length: {value:?}")))?;
            match usize::try_from(size) {
                Ok(size) if size <= MAX_MESSAGE => length = Some(size),
                _ => {
                    return Err(invalid(format!(
                        "a message of {size} bytes is larger than the {MAX_MESSAGE}-byte limit"
                    )));
                }
            }
        }
    }
    let length = length.ok_or_else(|| invalid("no Content-Length".to_owned()))?;
    let mut framed = format!("Content-Length: {length}\r\n\r\n").into_bytes();
    let head = framed.len();
    framed.resize(head + length, 0);
    input.read_exact(&mut framed[head..])?;
    Message::read(&mut Cursor::new(framed))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(bytes: &[u8]) -> io::Result<Option<Message>> {
        read_message(&mut Cursor::new(bytes.to_vec()))
    }

    #[test]
    fn frames() {
        let body = r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#;
        let framed = format!("Content-Length: {}\r\nContent-Type: x\r\n\r\n{body}", body.len());
        let message = read(framed.as_bytes()).unwrap().unwrap();
        assert!(matches!(message, Message::Notification(n) if n.method == "initialized"));
        assert!(read(b"").unwrap().is_none());
    }

    #[test]
    fn refuses_what_it_cannot_hold() {
        // Far larger than any message: refused before allocating anything.
        let err = read(b"Content-Length: 99999999999999\r\n\r\n").unwrap_err();
        assert!(err.to_string().contains("larger than"), "{err}");
        let err = read(b"Content-Length: 999999999999999999999999\r\n\r\n").unwrap_err();
        assert!(err.to_string().contains("bad Content-Length"), "{err}");
        // An endless header line.
        let long = vec![b'a'; usize::try_from(MAX_HEADER_LINE).unwrap() + 10];
        assert!(read(&long).is_err());
        assert!(read(b"Content-Length: 2\n\n{}").is_err(), "LF-only framing");
        assert!(read(b"\r\n{}").is_err(), "no Content-Length");
        assert!(read(b"Content-Length: 5\r\n\r\nnope!").is_err(), "not JSON-RPC");
        // A body shorter than announced.
        assert!(read(b"Content-Length: 50\r\n\r\n{}").is_err());
    }
}
