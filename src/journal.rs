//! The journal: append-only, every entry chained to the one before by SHA-256.
//! Editing any past entry breaks every hash after it, and `valla journal` will say so.

use crate::json::{self, esc, Json};
use crate::{sha256, util};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

pub struct Entry {
    pub n: u64,
    pub ts: String,
    pub kind: String,
    pub data: BTreeMap<String, String>,
    pub prev: String,
    pub hash: String,
}

pub struct Journal {
    path: PathBuf,
}

fn canon(data: &BTreeMap<String, String>) -> String {
    let parts: Vec<String> = data.iter().map(|(k, v)| format!("{}:{}", esc(k), esc(v))).collect();
    format!("{{{}}}", parts.join(","))
}

fn digest(n: u64, ts: &str, kind: &str, data: &BTreeMap<String, String>, prev: &str) -> String {
    sha256::hex(format!("{}\n{}\n{}\n{}\n{}", prev, n, ts, kind, canon(data)).as_bytes())
}

impl Journal {
    pub fn open(house: &Path) -> Journal {
        Journal { path: house.join(".valla").join("journal.jsonl") }
    }

    pub fn entries(&self) -> Vec<Entry> {
        util::read(&self.path)
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|l| {
                let v = json::parse(l).ok()?;
                let mut data = BTreeMap::new();
                if let Some(Json::Obj(m)) = v.get("data") {
                    for (k, x) in m {
                        data.insert(k.clone(), x.as_str().unwrap_or("").to_string());
                    }
                }
                Some(Entry {
                    n: v.get("n")?.as_num()? as u64,
                    ts: v.get("ts")?.as_str()?.to_string(),
                    kind: v.get("kind")?.as_str()?.to_string(),
                    data,
                    prev: v.get("prev")?.as_str()?.to_string(),
                    hash: v.get("hash")?.as_str()?.to_string(),
                })
            })
            .collect()
    }

    /// Append an entry. Returns its number.
    pub fn write(&self, kind: &str, pairs: &[(&str, &str)]) -> u64 {
        let all = self.entries();
        let (n, prev) = match all.last() {
            Some(e) => (e.n + 1, e.hash.clone()),
            None => (1, "genesis".to_string()),
        };
        let ts = util::iso(util::now_secs());
        let data: BTreeMap<String, String> = pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        let hash = digest(n, &ts, kind, &data, &prev);
        let line = format!(
            "{{\"n\":{},\"ts\":{},\"kind\":{},\"data\":{},\"prev\":{},\"hash\":{}}}\n",
            n,
            esc(&ts),
            esc(kind),
            canon(&data),
            esc(&prev),
            esc(&hash)
        );
        if let Some(d) = self.path.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        let mut f = std::fs::OpenOptions::new().create(true).append(true).open(&self.path).expect("journal");
        f.write_all(line.as_bytes()).expect("journal write");
        n
    }

    /// Ok(count) if the chain is unbroken, Err(n) at the first broken entry.
    pub fn verify(&self) -> Result<usize, u64> {
        let all = self.entries();
        let raw_lines = util::read(&self.path).lines().filter(|l| !l.trim().is_empty()).count();
        let mut prev = "genesis".to_string();
        for (i, e) in all.iter().enumerate() {
            if e.n != i as u64 + 1 || e.prev != prev || digest(e.n, &e.ts, &e.kind, &e.data, &e.prev) != e.hash {
                return Err(e.n);
            }
            prev = e.hash.clone();
        }
        if raw_lines != all.len() {
            return Err(all.len() as u64 + 1);
        }
        Ok(all.len())
    }

    /// The most recent approval or restoration whose sensors passed.
    pub fn last_known_good(&self) -> Option<Entry> {
        self.entries().into_iter().rev().find(|e| e.data.get("known_good").map(|s| s == "yes").unwrap_or(false))
    }

    pub fn get(&self, n: u64) -> Option<Entry> {
        self.entries().into_iter().find(|e| e.n == n)
    }
}
