//! The kitchen: whatever model the owner has connected. It cooks; it never serves.
//!
//! Speaks Ollama's native chat API (a URL ending in /api/chat) or the OpenAI-style chat API,
//! which covers most paid accounts. Settings come from ~/.config/valla/config (key = value)
//! or the environment:
//!   VALLA_KITCHEN_URL  default http://localhost:11434/api/chat
//!   VALLA_MODEL        default qwen3-coder:30b
//!   VALLA_CONTEXT      Ollama context size in tokens, default 32768
//!   VALLA_API_KEY      optional; sent as a Bearer token
//!   VALLA_KITCHEN=mock:<dir>  replay canned returns (dir/1.md, 2.md, ...) for tests

use crate::json::{self, esc};
use crate::util;
use std::path::{Path, PathBuf};

pub struct Kitchen {
    pub url: String,
    pub model: String,
    context: u32,
    key: Option<String>,
    mock: Option<PathBuf>,
}

pub struct Message {
    pub role: &'static str,
    pub content: String,
}

fn config_file() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".config/valla/config")
}

fn setting(key: &str, env: &str, default: &str) -> String {
    if let Ok(v) = std::env::var(env) {
        if !v.is_empty() {
            return v;
        }
    }
    util::read(&config_file())
        .lines()
        .filter_map(|l| l.split_once('='))
        .find(|(k, _)| k.trim() == key)
        .map(|(_, v)| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| default.to_string())
}

impl Kitchen {
    pub fn from_settings() -> Kitchen {
        let mock = std::env::var("VALLA_KITCHEN").ok().and_then(|v| v.strip_prefix("mock:").map(PathBuf::from));
        let key = setting("api_key", "VALLA_API_KEY", "");
        Kitchen {
            url: setting("kitchen_url", "VALLA_KITCHEN_URL", "http://localhost:11434/api/chat"),
            model: setting("model", "VALLA_MODEL", "qwen3-coder:30b"),
            context: setting("context", "VALLA_CONTEXT", "32768").parse().unwrap_or(32768),
            key: (!key.is_empty()).then_some(key),
            mock,
        }
    }

    pub fn describe(&self) -> String {
        match &self.mock {
            Some(d) => format!("mock kitchen at {}", d.display()),
            None => format!("{} at {}", self.model, self.url),
        }
    }

    /// Send the conversation; get back the kitchen's text and how long it took.
    pub fn cook(&self, msgs: &[Message], attempt: usize, scratch: &Path) -> Result<(String, f64), String> {
        if let Some(dir) = &self.mock {
            let p = dir.join(format!("{}.md", attempt));
            return std::fs::read_to_string(&p).map(|t| (t, 0.0)).map_err(|e| format!("mock {}: {}", p.display(), e));
        }
        let messages = msgs.iter().map(|m| format!("{{\"role\":{},\"content\":{}}}", esc(m.role), esc(&m.content))).collect::<Vec<_>>().join(",");
        // Ollama's native endpoint lets us set the context size; its OpenAI-style one silently truncates.
        let body = if self.url.ends_with("/api/chat") {
            format!(
                "{{\"model\":{},\"stream\":false,\"messages\":[{}],\"options\":{{\"temperature\":0,\"seed\":7,\"num_ctx\":{}}}}}",
                esc(&self.model),
                messages,
                self.context
            )
        } else {
            format!("{{\"model\":{},\"temperature\":0,\"seed\":7,\"stream\":false,\"messages\":[{}]}}", esc(&self.model), messages)
        };
        let req = scratch.join("kitchen-request.json");
        util::write(&req, &body);
        let mut args: Vec<String> = vec![
            "-sS".into(),
            "--max-time".into(),
            "3600".into(),
            "-H".into(),
            "Content-Type: application/json".into(),
        ];
        // the key goes in a file, not on the command line where other processes could read it
        let hdr = scratch.join("kitchen-auth.hdr");
        if let Some(k) = &self.key {
            util::write(&hdr, &format!("Authorization: Bearer {}\n", k));
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&hdr, std::fs::Permissions::from_mode(0o600));
            }
            args.push("-H".into());
            args.push(format!("@{}", hdr.display()));
        }
        args.push("--data-binary".into());
        args.push(format!("@{}", req.display()));
        args.push(self.url.clone());
        let started = std::time::Instant::now();
        let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let out = util::run(scratch, "curl", &refs);
        let _ = std::fs::remove_file(&req);
        let _ = std::fs::remove_file(&hdr);
        let secs = started.elapsed().as_secs_f64();
        if !out.ok {
            return Err(out.text);
        }
        let v = json::parse(&out.text).map_err(|e| format!("unreadable reply ({}): {}", e, out.text.chars().take(300).collect::<String>()))?;
        if let Some(err) = v.get("error") {
            return Err(format!("kitchen error: {:?}", err));
        }
        let text = v
            .get("choices")
            .and_then(|c| c.idx(0))
            .and_then(|c| c.get("message"))
            .or_else(|| v.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .ok_or_else(|| format!("no content in reply: {}", out.text.chars().take(300).collect::<String>()))?;
        Ok((text.to_string(), secs))
    }

    /// A short test: does the kitchen answer, and how fast?
    pub fn ping(&self, scratch: &Path) -> Result<String, String> {
        if self.mock.is_some() {
            return Ok("a mock kitchen".into());
        }
        let msgs = [Message { role: "user", content: "Reply with the single word: ready".into() }];
        let (t, secs) = self.cook(&msgs, 1, scratch)?;
        Ok(format!("answering ({:?}, {:.0}s)", t.trim().chars().take(20).collect::<String>(), secs))
    }
}
