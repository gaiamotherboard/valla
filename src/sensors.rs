//! Sensors: the only things Valla calls true.
//!
//! A house declares its sensors in `valla.sensors`, one per line:  name: shell command
//! A sensor passes when its command exits 0. If the house has a flake.nix and Nix is
//! present, each sensor runs inside `nix develop`, so the instruments are pinned too.

use crate::util;
use std::path::Path;
use std::time::Instant;

pub struct Reading {
    pub name: String,
    pub pass: bool,
    pub detail: String,
    pub secs: f64,
}

pub fn declared(dir: &Path) -> Vec<(String, String)> {
    util::read(&dir.join("valla.sensors"))
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| l.split_once(':').map(|(a, b)| (a.trim().to_string(), b.trim().to_string())))
        .collect()
}

fn pinned(dir: &Path) -> bool {
    dir.join("flake.nix").exists() && std::env::var("VALLA_NO_NIX").is_err() && util::run(dir, "nix", &["--version"]).ok
}

pub fn run_all(dir: &Path) -> Vec<Reading> {
    let use_nix = pinned(dir);
    declared(dir)
        .into_iter()
        .map(|(name, cmd)| {
            let t = Instant::now();
            let out = if use_nix {
                util::run(dir, "nix", &["develop", "--quiet", "--command", "sh", "-c", &cmd])
            } else {
                util::sh(dir, &cmd)
            };
            let tail: Vec<&str> = out.text.lines().rev().take(4).collect();
            let detail = tail.into_iter().rev().collect::<Vec<_>>().join(" | ");
            Reading { name, pass: out.ok, detail: if out.ok { String::new() } else { format!("exit {}: {}", out.code, detail) }, secs: t.elapsed().as_secs_f64() }
        })
        .collect()
}

pub fn summary(rs: &[Reading]) -> String {
    if rs.is_empty() {
        return "none declared".into();
    }
    let passed = rs.iter().filter(|r| r.pass).count();
    if passed == rs.len() {
        format!("all {} pass", rs.len())
    } else {
        format!("{} of {} FAIL", rs.len() - passed, rs.len())
    }
}

pub fn all_pass(rs: &[Reading]) -> bool {
    !rs.is_empty() && rs.iter().all(|r| r.pass)
}

pub fn report(rs: &[Reading]) -> String {
    rs.iter()
        .map(|r| format!("  {} {:<14} {:>5.1}s {}", if r.pass { "pass" } else { "FAIL" }, r.name, r.secs, r.detail))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Readings from the machine itself, for the doctor.
pub fn machine(dir: &Path) -> Vec<(String, String)> {
    let mut v = Vec::new();
    v.push(("host".into(), util::sh(dir, "hostname").text));
    v.push(("cpu".into(), util::sh(dir, "grep -m1 'model name' /proc/cpuinfo | cut -d: -f2 | sed 's/^ //'").text + &format!(" x{}", std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1))));
    v.push(("ram".into(), util::sh(dir, "awk '/MemTotal/ {printf \"%.0f GB\", $2/1048576}' /proc/meminfo").text));
    v.push(("disk free".into(), util::sh(dir, "df -h --output=avail . | tail -1").text.trim().to_string()));
    let t = util::sh(dir, "cat /sys/class/thermal/thermal_zone*/temp 2>/dev/null | sort -n | tail -1").text;
    if let Ok(m) = t.trim().parse::<f64>() {
        v.push(("hottest zone".into(), format!("{:.0} C", m / 1000.0)));
    }
    let gpu = util::sh(dir, "lspci 2>/dev/null | grep -Ei 'vga|3d|display' | cut -d: -f3 | sed 's/^ //'").text;
    v.push(("gpu".into(), if gpu.is_empty() { "none found".into() } else { gpu.replace('\n', "; ") }));
    v
}
