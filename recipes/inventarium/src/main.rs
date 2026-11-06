//! Inventarium: a small inventory tracker. No dependencies; one file of data.
//!
//!   inventarium [--port 8080] [--data inventarium.tsv] [--title "Inventarium"]

mod page;
mod store;

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use store::Store;

struct Config {
    port: u16,
    data: PathBuf,
    title: String,
}

fn config() -> Config {
    let mut c = Config { port: 8080, data: PathBuf::from("inventarium.tsv"), title: "Inventarium".into() };
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        let v = args.get(i + 1).cloned().unwrap_or_default();
        match args[i].as_str() {
            "--port" => c.port = v.parse().unwrap_or(8080),
            "--data" => c.data = PathBuf::from(v),
            "--title" => c.title = v,
            _ => {}
        }
        i += 2;
    }
    c
}

/// Decode application/x-www-form-urlencoded text.
fn decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < b.len() => {
                match u8::from_str_radix(std::str::from_utf8(&b[i + 1..i + 3]).unwrap_or("zz"), 16) {
                    Ok(v) => {
                        out.push(v);
                        i += 2;
                    }
                    Err(_) => out.push(b'%'),
                }
            }
            c => out.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

fn field(form: &str, key: &str) -> String {
    form.split('&').filter_map(|kv| kv.split_once('=')).find(|(k, _)| *k == key).map(|(_, v)| decode(v)).unwrap_or_default()
}

fn respond(stream: &mut TcpStream, status: &str, headers: &str, body: &str) {
    let _ = write!(stream, "HTTP/1.1 {}\r\nContent-Length: {}\r\nConnection: close\r\n{}\r\n{}", status, body.len(), headers, body);
}

fn handle(mut stream: TcpStream, cfg: &Config) {
    let mut reader = BufReader::new(stream.try_clone().expect("clone"));
    let mut first = String::new();
    if reader.read_line(&mut first).is_err() {
        return;
    }
    let mut len = 0usize;
    loop {
        let mut h = String::new();
        if reader.read_line(&mut h).is_err() || h.trim().is_empty() {
            break;
        }
        if let Some((k, v)) = h.split_once(':') {
            if k.trim().eq_ignore_ascii_case("content-length") {
                len = v.trim().parse().unwrap_or(0).min(64 * 1024);
            }
        }
    }
    let mut body = vec![0u8; len];
    let _ = reader.read_exact(&mut body);
    let body = String::from_utf8_lossy(&body).to_string();

    let mut parts = first.split_whitespace();
    let method = parts.next().unwrap_or("");
    let target = parts.next().unwrap_or("/");
    let (path, query) = target.split_once('?').unwrap_or((target, ""));

    let mut store = Store::open(&cfg.data);
    match (method, path) {
        ("GET", "/") => {
            let q = field(query, "q");
            let notice = field(query, "notice");
            let found = store.search(&q);
            let html = page::render(&cfg.title, &found, &q, store.items.len(), &notice);
            respond(&mut stream, "200 OK", "Content-Type: text/html; charset=utf-8\r\n", &html);
        }
        ("POST", "/add") => {
            let loc = match store.add(&field(&body, "name"), &field(&body, "status"), &field(&body, "location")) {
                Ok(_) => "/".to_string(),
                Err(e) => format!("/?notice={}", e.to_string().replace(' ', "+")),
            };
            respond(&mut stream, "303 See Other", &format!("Location: {}\r\n", loc), "");
        }
        ("POST", "/status") => {
            let id = field(&body, "id").parse().unwrap_or(0);
            let _ = store.set_status(id, &field(&body, "status"));
            respond(&mut stream, "303 See Other", "Location: /\r\n", "");
        }
        _ => respond(&mut stream, "404 Not Found", "Content-Type: text/plain\r\n", "not found"),
    }
}

fn main() {
    let cfg = config();
    let listener = TcpListener::bind(("0.0.0.0", cfg.port)).unwrap_or_else(|e| {
        eprintln!("cannot listen on port {}: {}", cfg.port, e);
        std::process::exit(1);
    });
    println!("{} on http://localhost:{}  (data: {})", cfg.title, cfg.port, cfg.data.display());
    for stream in listener.incoming().flatten() {
        handle(stream, &cfg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decodes_forms() {
        assert_eq!(decode("Shelf+A%32"), "Shelf A2");
        assert_eq!(field("name=Dell%20OptiPlex&status=sold", "name"), "Dell OptiPlex");
        assert_eq!(decode("100%"), "100%");
    }
}
