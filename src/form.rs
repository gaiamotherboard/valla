//! The fixed form every kitchen return must take, and the reader for it.
//!
//! ## EXEGESIS      E1: "exact quote" @ request        (or @ path/to/file)
//! ## HERMENEUTICS  H1 [E1]: what I take it to mean
//! ## SILENCE       S1: what the record does not say    (or: none)
//! ## PLAN          P1 [E1,H1]: a step, naming every file it touches
//! ## CHANGES       ### FILE path  + fenced full content,  or  ### DELETE path

pub const SECTIONS: [&str; 5] = ["EXEGESIS", "HERMENEUTICS", "SILENCE", "PLAN", "CHANGES"];

#[derive(Debug, Clone)]
pub struct Quote {
    pub id: String,
    pub text: String,
    pub source: String,
}

#[derive(Debug, Clone)]
pub struct Cited {
    pub id: String,
    pub cites: Vec<String>,
    pub text: String,
}

#[derive(Debug, Clone)]
pub enum Change {
    Write { path: String, content: String },
    Delete { path: String },
}

impl Change {
    pub fn path(&self) -> &str {
        match self {
            Change::Write { path, .. } | Change::Delete { path } => path,
        }
    }
}

#[derive(Debug, Default)]
pub struct Form {
    pub present: Vec<String>,
    pub exegesis: Vec<Quote>,
    pub hermeneutics: Vec<Cited>,
    pub silence: Vec<String>,
    pub plan: Vec<Cited>,
    pub changes: Vec<Change>,
    pub malformed: Vec<(String, String)>,
}

/// Strip a model's private thinking, if it left any.
pub fn clean(raw: &str) -> String {
    let mut s = raw.to_string();
    while let (Some(a), Some(b)) = (s.find("<think>"), s.find("</think>")) {
        if b < a {
            break;
        }
        s.replace_range(a..b + "</think>".len(), "");
    }
    s
}

fn split_sections(text: &str) -> Vec<(String, Vec<String>)> {
    let mut out: Vec<(String, Vec<String>)> = Vec::new();
    for line in text.lines() {
        let t = line.trim();
        if let Some(h) = t.strip_prefix("## ") {
            let name = h.trim().trim_matches('#').trim().to_uppercase();
            if SECTIONS.contains(&name.as_str()) {
                out.push((name, Vec::new()));
                continue;
            }
        }
        if let Some(last) = out.last_mut() {
            last.1.push(line.to_string());
        }
    }
    out
}

fn parse_id_cites(line: &str) -> Option<(String, Vec<String>, String)> {
    let line = line.trim().trim_start_matches(['-', '*']).trim();
    let (head, rest) = line.split_once(':')?;
    let head = head.trim();
    let (id, cites) = match head.split_once('[') {
        Some((id, c)) => {
            let c = c.trim_end_matches(']');
            (id.trim().to_string(), c.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect())
        }
        None => (head.to_string(), Vec::new()),
    };
    if id.is_empty() || id.contains(' ') {
        return None;
    }
    Some((id, cites, rest.trim().to_string()))
}

pub fn parse(raw: &str) -> Form {
    let text = clean(raw);
    let mut f = Form::default();
    for (name, lines) in split_sections(&text) {
        f.present.push(name.clone());
        match name.as_str() {
            "EXEGESIS" => {
                for l in lines.iter().filter(|l| !l.trim().is_empty()) {
                    let parsed = (|| {
                        let (id, _, rest) = parse_id_cites(l)?;
                        let first = rest.find('"')?;
                        let at = rest.rfind(" @ ")?;
                        let qend = rest[..at].rfind('"')?;
                        if qend <= first {
                            return None;
                        }
                        Some(Quote {
                            id,
                            text: rest[first + 1..qend].to_string(),
                            source: rest[at + 3..].trim().trim_matches('`').to_string(),
                        })
                    })();
                    match parsed {
                        Some(q) => f.exegesis.push(q),
                        None => f.malformed.push(("EXEGESIS".into(), l.trim().to_string())),
                    }
                }
            }
            "HERMENEUTICS" | "PLAN" => {
                for l in lines.iter().filter(|l| !l.trim().is_empty()) {
                    match parse_id_cites(l) {
                        Some((id, cites, text)) => {
                            let c = Cited { id, cites, text };
                            if name == "PLAN" {
                                f.plan.push(c)
                            } else {
                                f.hermeneutics.push(c)
                            }
                        }
                        None => f.malformed.push((name.clone(), l.trim().to_string())),
                    }
                }
            }
            "SILENCE" => {
                for l in lines.iter().filter(|l| !l.trim().is_empty()) {
                    let t = l.trim().trim_start_matches(['-', '*']).trim();
                    if t.eq_ignore_ascii_case("none") {
                        continue;
                    }
                    let t = match parse_id_cites(t) {
                        Some((_, _, rest)) => rest,
                        None => t.to_string(),
                    };
                    f.silence.push(t);
                }
            }
            "CHANGES" => f.changes = parse_changes(&lines),
            _ => {}
        }
    }
    f
}

fn parse_changes(lines: &[String]) -> Vec<Change> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let t = lines[i].trim();
        if let Some(p) = t.strip_prefix("### DELETE") {
            out.push(Change::Delete { path: p.trim().trim_matches('`').to_string() });
            i += 1;
            continue;
        }
        if let Some(p) = t.strip_prefix("### FILE") {
            let path = p.trim().trim_matches('`').to_string();
            // the block runs to the next header; content sits between the first fence and the last fence
            let mut j = i + 1;
            while j < lines.len() && !lines[j].trim().starts_with("### FILE") && !lines[j].trim().starts_with("### DELETE") {
                j += 1;
            }
            let block = &lines[i + 1..j];
            let open = block.iter().position(|l| l.trim_start().starts_with("```"));
            let close = block.iter().rposition(|l| l.trim() == "```");
            if let (Some(a), Some(b)) = (open, close) {
                if b > a {
                    let mut content = block[a + 1..b].join("\n");
                    content.push('\n');
                    out.push(Change::Write { path, content });
                }
            }
            i = j;
            continue;
        }
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    const SAMPLE: &str = r#"<think>hm</think>
## EXEGESIS
E1: "add a notes field" @ request
E2: "pub struct Item {" @ src/main.rs
## HERMENEUTICS
H1 [E1,E2]: Item gains a notes string.
## SILENCE
S1: Whether notes are searchable.
## PLAN
P1 [H1]: Add notes to Item in src/main.rs.
## CHANGES
### FILE src/main.rs
```rust
fn main() {}
```
"#;
    #[test]
    fn reads_the_form() {
        let f = parse(SAMPLE);
        assert_eq!(f.present.len(), 5);
        assert_eq!(f.exegesis[1].source, "src/main.rs");
        assert_eq!(f.exegesis[0].text, "add a notes field");
        assert_eq!(f.hermeneutics[0].cites, vec!["E1", "E2"]);
        assert_eq!(f.silence, vec!["Whether notes are searchable."]);
        assert_eq!(f.changes.len(), 1);
        if let Change::Write { content, .. } = &f.changes[0] {
            assert_eq!(content, "fn main() {}\n");
        }
    }
}
