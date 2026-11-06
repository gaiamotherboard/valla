//! The kitchen's declaration: what it can and cannot do, stated before it cooks.
//!
//! The default ships in kitchen/default.declaration. The owner may replace it with
//! ~/.config/valla/kitchen.declaration. Valla sends it with every brief, journals it
//! whenever it changes, and refuses any dish that writes a kind of file the kitchen
//! never declared it could.

use crate::util;
use std::path::PathBuf;

const BUILT_IN: &str = include_str!("../kitchen/default.declaration");

pub struct Declaration {
    pub text: String,
    pub source: String,
    pub files: Vec<String>,
    pub can: Vec<String>,
    pub cannot: Vec<String>,
    pub needs: Vec<String>,
}

fn own_file() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".config/valla/kitchen.declaration")
}

pub fn parse(text: &str, source: &str) -> Declaration {
    let mut d = Declaration { text: text.to_string(), source: source.to_string(), files: vec![], can: vec![], cannot: vec![], needs: vec![] };
    for l in text.lines().map(|l| l.trim()).filter(|l| !l.is_empty() && !l.starts_with('#')) {
        let Some((k, v)) = l.split_once(':') else { continue };
        let v = v.trim().to_string();
        match k.trim() {
            "files" => d.files.extend(v.split_whitespace().map(String::from)),
            "can" => d.can.push(v),
            "cannot" => d.cannot.push(v),
            "needs" => d.needs.push(v),
            _ => {}
        }
    }
    d
}

pub fn load() -> Declaration {
    let p = own_file();
    let own = util::read(&p);
    if own.trim().is_empty() {
        parse(BUILT_IN, "built in")
    } else {
        parse(&own, &p.display().to_string())
    }
}

impl Declaration {
    /// May this kitchen write the file at `path`?
    pub fn allows(&self, path: &str) -> bool {
        if self.files.is_empty() {
            return true;
        }
        let name = path.rsplit('/').next().unwrap_or(path);
        self.files.iter().any(|f| name == f || (f.starts_with('.') && name.ends_with(f.as_str()) && name.len() > f.len()) || name == f.trim_start_matches('.'))
    }

    pub fn fingerprint(&self) -> String {
        crate::sha256::hex(self.text.as_bytes())
    }

    pub fn for_kitchen(&self) -> String {
        let mut s = String::from("YOUR DECLARATION: this is what you have said you can and cannot do. Valla holds you to it.\n");
        s.push_str(&format!("- files you may write: {}\n", self.files.join(" ")));
        for c in &self.can {
            s.push_str(&format!("- can: {}\n", c));
        }
        for c in &self.cannot {
            s.push_str(&format!("- cannot: {}\n", c));
        }
        for c in &self.needs {
            s.push_str(&format!("- needs: {}\n", c));
        }
        s.push_str("If the request needs something you cannot do, say so under STANDARDS or SILENCE. Never serve it anyway.\n");
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn built_in_reads_and_allows() {
        let d = parse(BUILT_IN, "built in");
        assert!(!d.can.is_empty() && !d.cannot.is_empty() && !d.needs.is_empty());
        assert!(d.allows("src/main.rs"));
        assert!(d.allows("flake.nix"));
        assert!(d.allows(".gitignore"));
        assert!(d.allows("valla.sensors"));
        assert!(!d.allows("logo.png"));
        assert!(!d.allows("data.sqlite"));
    }
}
