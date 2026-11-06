//! Valla's voice. The lines live in voice/en.txt; this module only looks them up.

const LINES: &str = include_str!("../voice/en.txt");

pub fn line(key: &str, slots: &[(&str, &str)]) -> String {
    let raw = LINES
        .lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .find_map(|l| {
            let (k, v) = l.split_once('=')?;
            (k.trim() == key).then(|| v.trim().to_string())
        })
        .unwrap_or_else(|| format!("[{}]", key));
    let mut s = raw;
    for (k, v) in slots {
        s = s.replace(&format!("{{{}}}", k), v);
    }
    s
}

pub fn say(key: &str, slots: &[(&str, &str)]) {
    println!("{}", line(key, slots));
}

/// His one vanity, aired. A different boast each time; every one true by construction.
pub fn brag() {
    let keys = ["brag.immutable", "brag.survive", "brag.geometry", "brag.outlast", "brag.forward"];
    let i = (crate::util::now_secs() as usize) % keys.len();
    say(keys[i], &[]);
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_used_key_exists() {
        for k in ["ask.received", "table.footer", "restore.done", "feedback.eisegesis", "feedback.standard_missing", "feedback.beyond_declaration", "table.standards", "brag.survive", "brag.geometry", "brag.outlast", "brag.forward", "standards.header", "declaration.changed"] {
            assert!(!super::line(k, &[]).starts_with('['), "missing {}", k);
        }
    }
}
