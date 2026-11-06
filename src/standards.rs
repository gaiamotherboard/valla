//! The house standards: what every dish must answer to, for software and hardware alike.
//!
//! The list ships in standards/house.standards. A house may keep its own `valla.standards`.
//! Every kitchen return answers each standard under STANDARDS as met, n/a, or cannot,
//! and the corrector refuses a return that leaves one out or gives no reason.

use crate::util;
use std::path::Path;

const BUILT_IN: &str = include_str!("../standards/house.standards");

pub struct Standard {
    pub name: String,
    pub software: String,
    pub hardware: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    Met,
    NotApplying,
    Cannot,
}

impl Verdict {
    pub fn word(&self) -> &'static str {
        match self {
            Verdict::Met => "met",
            Verdict::NotApplying => "n/a",
            Verdict::Cannot => "cannot",
        }
    }
}

/// One line of a return's STANDARDS section, read.
#[derive(Debug, Clone)]
pub struct Answer {
    pub name: String,
    pub verdict: Option<Verdict>,
    pub reason: String,
}

pub fn parse(text: &str) -> Vec<Standard> {
    text.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| {
            let (name, body) = l.split_once(':')?;
            let (sw, hw) = body.split_once('|').unwrap_or((body, ""));
            Some(Standard { name: name.trim().to_lowercase(), software: sw.trim().to_string(), hardware: hw.trim().to_string() })
        })
        .collect()
}

pub fn load(house: &Path) -> Vec<Standard> {
    let own = util::read(&house.join("valla.standards"));
    let list = parse(&own);
    if list.is_empty() {
        parse(BUILT_IN)
    } else {
        list
    }
}

/// Read `name: verdict, reason`.
pub fn read_answer(line: &str) -> Option<Answer> {
    let t = line.trim().trim_start_matches(['-', '*']).trim();
    let (name, rest) = t.split_once(':')?;
    let name = name.trim().trim_matches('*').trim().to_lowercase();
    if name.is_empty() || name.contains(' ') {
        return None;
    }
    let rest = rest.trim();
    let lower = rest.to_lowercase();
    let (verdict, after) = if lower.starts_with("met") {
        (Some(Verdict::Met), &rest[3..])
    } else if lower.starts_with("n/a") {
        (Some(Verdict::NotApplying), &rest[3..])
    } else if lower.starts_with("does not apply") {
        (Some(Verdict::NotApplying), &rest[14..])
    } else if lower.starts_with("cannot") {
        (Some(Verdict::Cannot), &rest[6..])
    } else {
        (None, rest)
    };
    let reason = after.trim().trim_start_matches([',', ':', '-', ';']).trim().to_string();
    Some(Answer { name, verdict, reason })
}

/// The standards as the kitchen should see them.
pub fn for_kitchen(list: &[Standard]) -> String {
    let mut s = String::from("HOUSE STANDARDS: answer every one under ## STANDARDS as  name: met, how  |  name: n/a, why  |  name: cannot, why\n");
    for st in list {
        s.push_str(&format!("- {}: software: {}. hardware: {}.\n", st.name, st.software, st.hardware));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn built_in_list_reads() {
        let l = parse(BUILT_IN);
        assert_eq!(l.len(), 7);
        assert_eq!(l[0].name, "best-practice");
        assert!(l.iter().any(|s| s.name == "local-first" && !s.hardware.is_empty()));
    }
    #[test]
    fn answers_read() {
        let a = read_answer("backup: met, data file is copied nightly by the sensor").unwrap();
        assert_eq!(a.verdict, Some(Verdict::Met));
        assert_eq!(a.reason, "data file is copied nightly by the sensor");
        let b = read_answer("- battery-backup: n/a - no data is written").unwrap();
        assert_eq!(b.verdict, Some(Verdict::NotApplying));
        assert_eq!(b.reason, "no data is written");
        let c = read_answer("redundancy: maybe").unwrap();
        assert_eq!(c.verdict, None);
    }
}
