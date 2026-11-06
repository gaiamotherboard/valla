//! The corrector's duty. Every check here is plain code: no model is asked whether it behaved.

use crate::form::{Form, SECTIONS};
use crate::declaration::Declaration;
use crate::standards::{Standard, Verdict};
use crate::{util, voice};
use std::collections::HashSet;
use std::path::Path;

pub struct Fault {
    pub rule: &'static str,
    pub message: String,
}

fn fault(rule: &'static str, key: &str, slots: &[(&str, &str)]) -> Fault {
    Fault { rule, message: voice::line(key, slots) }
}

fn safe_path(p: &str) -> bool {
    !p.is_empty()
        && !p.starts_with('/')
        && !p.split('/').any(|s| s == ".." || s == ".git" || s == ".valla")
        && !p.contains('\\')
}

/// Inspect a return. `house` is the committed state the kitchen was shown.
pub fn inspect(form: &Form, request: &str, house: &Path, standards: &[Standard], decl: &Declaration) -> Vec<Fault> {
    let mut faults = Vec::new();

    for s in SECTIONS {
        if !form.present.iter().any(|p| p == s) {
            faults.push(fault("missing_section", "feedback.missing_section", &[("section", s)]));
        }
    }
    for (sec, line) in &form.malformed {
        faults.push(fault("malformed", "feedback.malformed", &[("id", sec), ("line", line)]));
    }

    // Exegesis: every quote must exist, verbatim (whitespace aside), in its stated source.
    let mut ids: HashSet<String> = HashSet::new();
    for q in &form.exegesis {
        ids.insert(q.id.clone());
        let source_text = if q.source.eq_ignore_ascii_case("request") {
            Some(request.to_string())
        } else if safe_path(&q.source) {
            let o = util::run(house, "git", &["show", &format!("HEAD:{}", q.source)]);
            o.ok.then_some(o.text)
        } else {
            None
        };
        match source_text {
            None => faults.push(fault("bad_source", "feedback.bad_source", &[("id", &q.id), ("source", &q.source)])),
            Some(t) => {
                if q.text.trim().is_empty() || !util::squash(&t).contains(&util::squash(&q.text)) {
                    faults.push(fault("bad_quote", "feedback.bad_quote", &[("id", &q.id), ("quote", &q.text), ("source", &q.source)]));
                }
            }
        }
    }

    // Hermeneutics: each reading cites exegesis.
    for h in &form.hermeneutics {
        if h.cites.is_empty() {
            faults.push(fault("no_cite", "feedback.no_cite", &[("id", &h.id)]));
        }
        for c in &h.cites {
            if !ids.contains(c) {
                faults.push(fault("dead_cite", "feedback.dead_cite", &[("id", &h.id), ("cite", c)]));
            }
        }
    }
    for h in &form.hermeneutics {
        ids.insert(h.id.clone());
    }

    // Plan: each step cites exegesis or hermeneutics.
    for p in &form.plan {
        if p.cites.is_empty() {
            faults.push(fault("no_cite", "feedback.no_cite", &[("id", &p.id)]));
        }
        for c in &p.cites {
            if !ids.contains(c) {
                faults.push(fault("dead_cite", "feedback.dead_cite", &[("id", &p.id), ("cite", c)]));
            }
        }
    }

    // Changes: safe paths only, and every path touched must be named in the plan.
    let plan_text: String = form.plan.iter().map(|p| p.text.as_str()).collect::<Vec<_>>().join("\n");
    for ch in &form.changes {
        let path = ch.path();
        if !safe_path(path) {
            faults.push(fault("bad_path", "feedback.bad_path", &[("path", path)]));
            continue;
        }
        if !plan_text.contains(path) {
            faults.push(fault("eisegesis", "feedback.eisegesis", &[("path", path)]));
        }
    }
    // The kitchen's declaration: a dish it said it cannot make is refused unopened.
    for ch in &form.changes {
        if safe_path(ch.path()) && !decl.allows(ch.path()) {
            faults.push(fault("beyond_declaration", "feedback.beyond_declaration", &[("path", ch.path())]));
        }
    }

    // House standards: each answered as met, n/a or cannot; the last two with a reason.
    if form.present.iter().any(|p| p == "STANDARDS") {
        for st in standards {
            match form.standards.iter().find(|a| a.name == st.name) {
                None => faults.push(fault("standard_missing", "feedback.standard_missing", &[("name", &st.name)])),
                Some(a) => match &a.verdict {
                    None => faults.push(fault("standard_verdict", "feedback.standard_verdict", &[("name", &st.name)])),
                    Some(v) if a.reason.is_empty() => faults.push(fault("standard_reason", "feedback.standard_reason", &[("name", &st.name), ("verdict", v.word())])),
                    Some(Verdict::Met) | Some(Verdict::NotApplying) | Some(Verdict::Cannot) => {}
                },
            }
        }
    }
    if !form.plan.is_empty() && form.changes.is_empty() && form.present.iter().any(|p| p == "CHANGES") {
        faults.push(fault("no_changes", "feedback.no_changes", &[]));
    }
    faults
}
