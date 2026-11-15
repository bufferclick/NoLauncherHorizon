//! The window's server: loopback only, one random word a request must carry, and no framework.
//!
//! FH's window is a page, so something has to answer the browser: this. It listens on `127.0.0.1`,
//! a port the system gives it, and refuses every request that does not carry the token it was
//! started with — the token is in the address the browser was opened at, so the only pages that
//! can address it are the ones this program drew. That is what keeps a page on the internet from
//! asking it to start a game.
//!
//! HTTP/1.1 as a browser asks it, and only as much of it as the window needs: a request line, the
//! headers to the blank line, a path and its query. One thread per connection, because a browser
//! opens several at once for one page.

use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// What a request asked for.
pub struct Request {
    pub method: String,
    /// The path without its query, percent-decoded (`/`, `/launch`, `/icon`).
    pub path: String,
    /// The query's pairs, percent-decoded: `k` is the token, `id` what a row names.
    pub query: Vec<(String, String)>,
}

impl Request {
    /// A query parameter, by name.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.query
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
}

/// What to answer.
pub struct Response {
    pub status: u16,
    pub content_type: &'static str,
    pub body: Vec<u8>,
    /// Where to go instead, for a redirect.
    pub location: Option<String>,
}

impl Response {
    pub fn html(body: String) -> Self {
        Self::page(200, body)
    }

    /// A page with a status of its own: the window's pages answer `200`, the one that says the
    /// address is not a page of this window answers `404`.
    pub fn page(status: u16, body: String) -> Self {
        Self {
            status,
            content_type: "text/html; charset=utf-8",
            body: body.into_bytes(),
            location: None,
        }
    }

    pub fn svg(body: &str) -> Self {
        Self {
            status: 200,
            content_type: "image/svg+xml",
            body: body.as_bytes().to_vec(),
            location: None,
        }
    }

    /// A page saying no: what the sender did wrong, in the plainest words.
    pub fn text(status: u16, what: &str) -> Self {
        Self {
            status,
            content_type: "text/plain; charset=utf-8",
            body: format!("{what}\n").into_bytes(),
            location: None,
        }
    }

    /// Send the sender elsewhere (`303`, so the browser asks again with a plain GET).
    pub fn redirect(to: String) -> Self {
        Self {
            status: 303,
            content_type: "text/plain; charset=utf-8",
            body: format!("{to}\n").into_bytes(),
            location: Some(to),
        }
    }
}

/// The running server: where it listens and the word its pages carry.
pub struct Server {
    pub port: u16,
    pub token: String,
}

impl Server {
    /// The address the browser is opened at.
    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}/?k={}", self.port, self.token)
    }

    /// Serves `handle` on a free loopback port until the process ends. `handle` is called on its
    /// own thread for every connection, so it must answer on its own (the window's state is
    /// shared behind a lock).
    pub fn start<F>(token: String, handle: F) -> Result<Self, String>
    where
        F: Fn(&Request) -> Response + Send + Sync + 'static,
    {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .map_err(|e| format!("Could not open a port for the window: {e}"))?;
        let port = listener
            .local_addr()
            .map_err(|e| format!("Could not read the window's port: {e}"))?
            .port();
        let handle = Arc::new(handle);
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                let handle = handle.clone();
                std::thread::spawn(move || serve_one(stream, handle.as_ref()));
            }
        });
        Ok(Self { port, token })
    }
}

/// Answers one connection: one request, one response, then closed.
fn serve_one<F>(stream: TcpStream, handle: &F)
where
    F: Fn(&Request) -> Response,
{
    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(10)));
    let mut writer = match stream.try_clone() {
        Ok(writer) => writer,
        Err(_) => return,
    };
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    if reader.read_line(&mut line).is_err() {
        return;
    }
    // The rest of the request's headers, up to the blank line: nothing here reads them, but the
    // connection is closed politely only after they are gone.
    let mut header = String::new();
    let mut guard = 0;
    while reader.read_line(&mut header).map(|n| n > 0).unwrap_or(false) {
        let trimmed = header.trim_end();
        if trimmed.is_empty() {
            break;
        }
        guard += 1;
        if guard > 100 {
            break;
        }
        header.clear();
    }
    let request = match parse(&line) {
        Some(request) => request,
        None => {
            write_response(&mut writer, Response::text(400, "Only HTTP requests here."));
            return;
        }
    };
    let response = handle(&request);
    write_response(&mut writer, response);
}

/// A request line (`GET /?k=x HTTP/1.1`) as a [`Request`].
pub fn parse(line: &str) -> Option<Request> {
    let mut parts = line.split_whitespace();
    let method = parts.next()?.to_owned();
    let target = parts.next()?;
    let (path, query) = match target.split_once('?') {
        Some((path, query)) => (path, query),
        None => (target, ""),
    };
    Some(Request {
        method,
        path: decode(path),
        query: query
            .split('&')
            .filter(|pair| !pair.is_empty())
            .map(|pair| match pair.split_once('=') {
                Some((key, value)) => (decode(key), decode(value)),
                None => (decode(pair), String::new()),
            })
            .collect(),
    })
}

/// `text` with `%XX` and `+` undone: a query's and a path's way of writing a space and a slash.
pub fn decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        match bytes[at] {
            b'%' if at + 2 < bytes.len() => {
                match u8::from_str_radix(&text[at + 1..at + 3], 16) {
                    Ok(byte) => {
                        out.push(byte);
                        at += 3;
                    }
                    Err(_) => {
                        out.push(bytes[at]);
                        at += 1;
                    }
                }
            }
            b'+' => {
                out.push(b' ');
                at += 1;
            }
            byte => {
                out.push(byte);
                at += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Writes the answer, and closes the connection: every page here is complete in one response.
fn write_response(stream: &mut TcpStream, response: Response) {
    let reason = match response.status {
        200 => "OK",
        303 => "See Other",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        _ => "OK",
    };
    let mut head = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\n\
         Cache-Control: no-store\r\nConnection: close\r\n",
        response.status,
        reason,
        response.content_type,
        response.body.len()
    );
    if let Some(location) = &response.location {
        head.push_str(&format!("Location: {location}\r\n"));
    }
    head.push_str("\r\n");
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(&response.body);
    let _ = stream.flush();
}

/// A word no other page can guess: the token in the address the browser is opened at.
pub fn token() -> String {
    // The clock alone is not enough: two calls in one tick of it would be one word, so a counter
    // goes in as well (this program makes one per run, and the tests make more).
    static MADE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let made = MADE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_nanos())
        .unwrap_or(0);
    let mut seed = (nanos as u64)
        ^ (std::process::id() as u64).rotate_left(32)
        ^ made.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    let mut word = String::with_capacity(32);
    for _ in 0..32 {
        // xorshift64*: not a cryptography library, a word a page on the internet cannot guess.
        seed ^= seed >> 12;
        seed ^= seed << 25;
        seed ^= seed >> 27;
        let value = seed.wrapping_mul(0x2545_F491_4F6C_DD1D);
        word.push(b"0123456789abcdef"[(value >> 60) as usize] as char);
    }
    word
}

#[cfg(test)]
mod tests {
    use super::{decode, parse, token, Request, Response, Server};
    use std::io::{BufRead, BufReader, Write};

    #[test]
    fn a_request_line_becomes_its_path_and_its_query() {
        let request = parse("GET /icon?k=abc&id=instances%2Fmy-pack HTTP/1.1").unwrap();
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/icon");
        assert_eq!(request.get("k"), Some("abc"));
        assert_eq!(request.get("id"), Some("instances/my-pack"));
        let bare = parse("GET / HTTP/1.1").unwrap();
        assert_eq!(bare.path, "/");
        assert_eq!(bare.get("k"), None);
        assert!(parse("nonsense").is_none());
        assert_eq!(decode("a%20b+c"), "a b c");
        assert_eq!(decode("100%"), "100%");
    }

    /// The server answers its own pages and refuses everything else: no token, no answer.
    #[test]
    fn the_server_answers_only_requests_that_carry_the_token() {
        let word = token();
        assert_eq!(word.len(), 32);
        assert!(word.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(word, token());

        let asked = word.clone();
        let server = Server::start(word, move |request: &Request| {
            if request.get("k") != Some(asked.as_str()) {
                return Response::text(403, "No.");
            }
            Response::html(format!("<p>{}</p>", request.path))
        })
        .unwrap();
        assert!(server.port > 0);
        let with = ask(server.port, &format!("/?k={}", server.token));
        assert!(with.starts_with("HTTP/1.1 200"), "{with}");
        assert!(with.contains("<p>/</p>"), "{with}");
        let without = ask(server.port, "/");
        assert!(without.starts_with("HTTP/1.1 403"), "{without}");
        assert!(server.url().contains(&server.token), "{}", server.url());
    }

    /// One request, one response, by hand: no client library on this side either.
    fn ask(port: u16, target: &str) -> String {
        let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
        write!(stream, "GET {target} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n").unwrap();
        let mut reader = BufReader::new(stream);
        let mut text = String::new();
        let mut line = String::new();
        while reader.read_line(&mut line).map(|n| n > 0).unwrap_or(false) {
            text.push_str(&line);
            line.clear();
        }
        text
    }
}
