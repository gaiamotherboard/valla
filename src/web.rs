//! `valla web`: the steward's chat, in a browser.
//!
//! The conversation is the journal itself, so nothing said to Valla is ever lost.
//! Every action runs the same `valla` commands as the terminal, one at a time,
//! and their output streams into the chat while they work.
//!
//!   valla web [folder] [--port 8090] [--bind 127.0.0.1]
//!
//! [folder] is one house, or a folder of houses (each a git repository).
//! It listens on localhost only unless --bind says otherwise. It has no login of its own:
//! put it behind something that does (a Cloudflare Access tunnel, for example) before
//! exposing it beyond this machine.

use crate::journal::Journal;
use crate::json::esc;
use crate::util;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

const PAGE: &str = include_str!("../web/index.html");

struct Job {
    house: String,
    label: String,
    log: PathBuf,
    child: Child,
    started: u64,
}

struct App {
    root: PathBuf,
    job: Mutex<Option<Job>>,
    last: Mutex<Option<(String, String, String)>>, // house, label, final output
}

fn houses(root: &Path) -> Vec<(String, PathBuf)> {
    if root.join(".git").exists() {
        let name = root.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "house".into());
        return vec![(name, root.to_path_buf())];
    }
    let mut v: Vec<(String, PathBuf)> = std::fs::read_dir(root)
        .map(|d| {
            d.filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.join(".git").exists())
                .map(|p| (p.file_name().unwrap().to_string_lossy().to_string(), p))
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

fn decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < b.len() => match u8::from_str_radix(std::str::from_utf8(&b[i + 1..i + 3]).unwrap_or("zz"), 16) {
                Ok(v) => {
                    out.push(v);
                    i += 2;
                }
                Err(_) => out.push(b'%'),
            },
            c => out.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

fn field(form: &str, key: &str) -> String {
    form.split('&').filter_map(|kv| kv.split_once('=')).find(|(k, _)| *k == key).map(|(_, v)| decode(v)).unwrap_or_default()
}

fn send(stream: &mut TcpStream, status: &str, ctype: &str, body: &str) {
    let _ = write!(
        stream,
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{}",
        status,
        ctype,
        body.len(),
        body
    );
}

fn json_str_map(pairs: &[(&str, String)]) -> String {
    format!("{{{}}}", pairs.iter().map(|(k, v)| format!("{}:{}", esc(k), v)).collect::<Vec<_>>().join(","))
}

impl App {
    fn house(&self, name: &str) -> Option<PathBuf> {
        houses(&self.root).into_iter().find(|(n, _)| n == name).map(|(_, p)| p)
    }

    /// Reap a finished job, keeping its output as the last result.
    fn reap(&self) {
        let mut job = self.job.lock().unwrap();
        let done = match job.as_mut() {
            Some(j) => matches!(j.child.try_wait(), Ok(Some(_))),
            None => false,
        };
        if done {
            let j = job.take().unwrap();
            let out = util::read(&j.log);
            let _ = std::fs::remove_file(&j.log);
            *self.last.lock().unwrap() = Some((j.house, j.label, out));
        }
    }

    fn start(&self, house_name: &str, label: &str, args: &[String]) -> Result<(), String> {
        self.reap();
        let house = self.house(house_name).ok_or("no such house")?;
        let mut job = self.job.lock().unwrap();
        if job.is_some() {
            return Err("I am already at work on something. One thing at a time.".into());
        }
        let log = std::env::temp_dir().join(format!("valla-web-{}-{}.log", std::process::id(), util::now_secs()));
        let f = std::fs::File::create(&log).map_err(|e| e.to_string())?;
        let f2 = f.try_clone().map_err(|e| e.to_string())?;
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let child = Command::new(exe)
            .args(args)
            .current_dir(&house)
            .stdin(Stdio::null())
            .stdout(f)
            .stderr(f2)
            .spawn()
            .map_err(|e| e.to_string())?;
        *job = Some(Job { house: house_name.to_string(), label: label.to_string(), log, child, started: util::now_secs() });
        *self.last.lock().unwrap() = None;
        Ok(())
    }

    fn state(&self, house_name: &str) -> String {
        self.reap();
        let list = houses(&self.root);
        let names = format!("[{}]", list.iter().map(|(n, _)| esc(n)).collect::<Vec<_>>().join(","));
        let house = list.iter().find(|(n, _)| n == house_name).or(list.first());
        let (hname, hpath) = match house {
            Some((n, p)) => (n.clone(), p.clone()),
            None => return json_str_map(&[("houses", names), ("house", "null".into())]),
        };
        let entries = Journal::open(&hpath).entries();
        let skip = entries.len().saturating_sub(400);
        let journal = entries[skip..]
            .iter()
            .map(|e| {
                let data = format!("{{{}}}", e.data.iter().map(|(k, v)| format!("{}:{}", esc(k), esc(v))).collect::<Vec<_>>().join(","));
                format!("{{\"n\":{},\"ts\":{},\"kind\":{},\"data\":{}}}", e.n, esc(&e.ts), esc(&e.kind), data)
            })
            .collect::<Vec<_>>()
            .join(",");
        let pending_dir = hpath.join(".valla/pending");
        let has_pending = std::fs::read_dir(&pending_dir).map(|mut d| d.next().is_some()).unwrap_or(false);
        let table = if has_pending {
            let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("valla"));
            esc(&util::run(&hpath, &exe.display().to_string(), &["table"]).text)
        } else {
            "null".into()
        };
        let job = match self.job.lock().unwrap().as_ref() {
            Some(j) => format!(
                "{{\"house\":{},\"label\":{},\"log\":{},\"seconds\":{}}}",
                esc(&j.house),
                esc(&j.label),
                esc(&util::read(&j.log)),
                util::now_secs().saturating_sub(j.started)
            ),
            None => "null".into(),
        };
        let last = match self.last.lock().unwrap().as_ref() {
            Some((h, l, o)) if *h == hname => format!("{{\"label\":{},\"output\":{}}}", esc(l), esc(o)),
            _ => "null".into(),
        };
        json_str_map(&[
            ("houses", names),
            ("house", esc(&hname)),
            ("journal", format!("[{}]", journal)),
            ("table", table),
            ("job", job),
            ("last", last),
        ])
    }
}

fn handle(mut stream: TcpStream, app: &App) {
    let mut reader = BufReader::new(match stream.try_clone() {
        Ok(s) => s,
        Err(_) => return,
    });
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
                len = v.trim().parse().unwrap_or(0).min(256 * 1024);
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

    match (method, path) {
        ("GET", "/") => send(&mut stream, "200 OK", "text/html; charset=utf-8", PAGE),
        ("GET", "/api/state") => {
            let s = app.state(&field(query, "house"));
            send(&mut stream, "200 OK", "application/json", &s)
        }
        ("POST", "/api/do") => {
            let house = field(&body, "house");
            let action = field(&body, "action");
            let text = field(&body, "text");
            let args: Option<(String, Vec<String>)> = match action.as_str() {
                "ask" if !text.trim().is_empty() => Some(("ask".into(), vec!["ask".into(), text.clone()])),
                "approve" => Some(("approve".into(), vec!["approve".into()])),
                "approve-anyway" => Some(("approve --anyway".into(), vec!["approve".into(), "--anyway".into()])),
                "decline" => Some(("decline".into(), vec!["decline".into()])),
                "restore" => Some(("restore".into(), vec!["restore".into()])),
                "diff" => Some(("diff".into(), vec!["diff".into()])),
                "words" => Some(("words".into(), vec!["words".into()])),
                "journal" => Some(("journal".into(), vec!["journal".into(), "40".into()])),
                "doctor" => Some(("doctor".into(), vec!["doctor".into()])),
                _ => None,
            };
            match args {
                None => send(&mut stream, "400 Bad Request", "application/json", "{\"error\":\"unknown action\"}"),
                Some((label, a)) => match app.start(&house, &label, &a) {
                    Ok(()) => send(&mut stream, "200 OK", "application/json", "{\"ok\":true}"),
                    Err(e) => send(&mut stream, "409 Conflict", "application/json", &format!("{{\"error\":{}}}", esc(&e))),
                },
            }
        }
        _ => send(&mut stream, "404 Not Found", "text/plain", "not found"),
    }
}

pub fn serve(args: &[String]) -> Result<(), String> {
    let mut root: Option<PathBuf> = None;
    let mut port: u16 = 8090;
    let mut bind = "127.0.0.1".to_string();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--port" => {
                port = args.get(i + 1).and_then(|s| s.parse().ok()).ok_or("--port wants a number")?;
                i += 1;
            }
            "--bind" => {
                bind = args.get(i + 1).cloned().ok_or("--bind wants an address")?;
                i += 1;
            }
            other => root = Some(PathBuf::from(other)),
        }
        i += 1;
    }
    let root = match root {
        Some(r) => r,
        None => crate::house::find_root().unwrap_or_else(|_| std::env::current_dir().unwrap_or_default()),
    };
    let root = std::fs::canonicalize(&root).map_err(|e| format!("{}: {}", root.display(), e))?;
    let found = houses(&root);
    if found.is_empty() {
        return Err(format!("I find no houses in {}. Point me at a house, or a folder of them.", root.display()));
    }
    let listener = TcpListener::bind((bind.as_str(), port)).map_err(|e| format!("cannot listen on {}:{}: {}", bind, port, e))?;
    println!(
        "At your service on http://{}:{}  ({} house{}: {})",
        if bind == "0.0.0.0" { "localhost" } else { &bind },
        port,
        found.len(),
        if found.len() == 1 { "" } else { "s" },
        found.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>().join(", ")
    );
    if bind != "127.0.0.1" && bind != "localhost" {
        println!("Note: I am listening beyond this machine and keep no login of my own. Put me behind one.");
    }
    let app = Arc::new(App { root, job: Mutex::new(None), last: Mutex::new(None) });
    for stream in listener.incoming().flatten() {
        let app = Arc::clone(&app);
        std::thread::spawn(move || handle(stream, &app));
    }
    Ok(())
}
