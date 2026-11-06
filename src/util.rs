//! Small helpers: running pinned tools, time, files.

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Out {
    pub ok: bool,
    pub code: i32,
    pub text: String,
}

/// Run a shell command in a directory. Output is stdout and stderr together.
pub fn sh(dir: &Path, cmd: &str) -> Out {
    match Command::new("sh").arg("-c").arg(cmd).current_dir(dir).stdin(Stdio::null()).output() {
        Ok(o) => {
            let mut t = String::from_utf8_lossy(&o.stdout).to_string();
            t.push_str(&String::from_utf8_lossy(&o.stderr));
            Out { ok: o.status.success(), code: o.status.code().unwrap_or(-1), text: t.trim_end().to_string() }
        }
        Err(e) => Out { ok: false, code: -1, text: e.to_string() },
    }
}

/// Run a program with explicit arguments (no shell).
pub fn run(dir: &Path, prog: &str, args: &[&str]) -> Out {
    match Command::new(prog).args(args).current_dir(dir).stdin(Stdio::null()).output() {
        Ok(o) => {
            let mut t = String::from_utf8_lossy(&o.stdout).to_string();
            t.push_str(&String::from_utf8_lossy(&o.stderr));
            Out { ok: o.status.success(), code: o.status.code().unwrap_or(-1), text: t.trim_end().to_string() }
        }
        Err(e) => Out { ok: false, code: -1, text: format!("{}: {}", prog, e) },
    }
}

pub fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// UTC timestamp as YYYY-MM-DDTHH:MM:SSZ.
pub fn iso(secs: u64) -> String {
    let days = (secs / 86400) as i64;
    let rem = secs % 86400;
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", y, m, d, rem / 3600, (rem % 3600) / 60, rem % 60)
}

pub fn read(p: &Path) -> String {
    std::fs::read_to_string(p).unwrap_or_default()
}

pub fn write(p: &Path, s: &str) {
    if let Some(d) = p.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    std::fs::write(p, s).unwrap_or_else(|e| panic!("cannot write {}: {}", p.display(), e));
}

/// Collapse all runs of whitespace to a single space, for quote matching.
pub fn squash(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    #[test]
    fn iso_known() {
        assert_eq!(super::iso(0), "1970-01-01T00:00:00Z");
        assert_eq!(super::iso(1_794_000_000), "2026-11-06T21:20:00Z");
    }
}
