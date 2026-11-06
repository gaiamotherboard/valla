//! The house lexicon: the proper words for this kind of work, and where each lives in the code.
//!
//! A house may keep a `valla.lexicon` file:
//!   word variable: A name that holds a value. @ world.py: speed: float = 360.0
//!   vague faster: Faster how? The cat's speed, the stars' fall speed, or how often stars appear?
//!
//! Valla uses it to teach precise asking: he flags vague words when you ask, shows on the
//! table which proper words were in play, sends the lexicon to the kitchen so its readings
//! use the same words, and keeps count of the words you have used yourself.

use crate::util;
use std::path::Path;

pub struct Word {
    pub term: String,
    pub meaning: String,
    pub place: String,
}

pub struct Vague {
    pub term: String,
    pub better: String,
}

#[derive(Default)]
pub struct Lexicon {
    pub words: Vec<Word>,
    pub vague: Vec<Vague>,
}

pub fn load(dir: &Path) -> Lexicon {
    parse(&util::read(&dir.join("valla.lexicon")))
}

pub fn parse(text: &str) -> Lexicon {
    let mut lx = Lexicon::default();
    for line in text.lines().map(|l| l.trim()).filter(|l| !l.is_empty() && !l.starts_with('#')) {
        let (kind, rest) = match line.split_once(' ') {
            Some(x) => x,
            None => continue,
        };
        let Some((term, body)) = rest.split_once(':') else { continue };
        let term = term.trim().to_string();
        let body = body.trim();
        match kind {
            "word" => {
                let (meaning, place) = match body.split_once(" @ ") {
                    Some((m, p)) => (m.trim().to_string(), p.trim().to_string()),
                    None => (body.to_string(), String::new()),
                };
                lx.words.push(Word { term, meaning, place });
            }
            "vague" => lx.vague.push(Vague { term, better: body.to_string() }),
            _ => {}
        }
    }
    lx
}

/// Does `text` mention `term` as a whole word or phrase (any case, simple plurals allowed)?
pub fn mentions(text: &str, term: &str) -> bool {
    let hay = text.to_lowercase();
    let needle = term.to_lowercase();
    if needle.is_empty() {
        return false;
    }
    let is_word = |c: Option<char>| c.map(|c| c.is_alphanumeric() || c == '_').unwrap_or(false);
    let mut start = 0;
    while let Some(pos) = hay[start..].find(&needle) {
        let a = start + pos;
        let b = a + needle.len();
        let before = hay[..a].chars().last();
        let rest = &hay[b..];
        let after_ok = !is_word(rest.chars().next()) || rest.starts_with("s") && !is_word(rest.chars().nth(1)) || rest.starts_with("es") && !is_word(rest.chars().nth(2));
        if !is_word(before) && after_ok {
            return true;
        }
        start = a + 1;
        while !hay.is_char_boundary(start) {
            start += 1;
        }
    }
    false
}

impl Lexicon {
    pub fn is_empty(&self) -> bool {
        self.words.is_empty() && self.vague.is_empty()
    }

    pub fn words_in(&self, text: &str) -> Vec<&Word> {
        self.words.iter().filter(|w| mentions(text, &w.term)).collect()
    }

    pub fn vague_in(&self, text: &str) -> Vec<&Vague> {
        self.vague.iter().filter(|v| mentions(text, &v.term)).collect()
    }

    /// The lexicon as the kitchen should see it.
    pub fn for_kitchen(&self) -> String {
        if self.words.is_empty() {
            return String::new();
        }
        let mut s = String::from(
            "HOUSE WORDS: the owner is learning these. In HERMENEUTICS and PLAN, use the proper word wherever it fits, so the owner sees it used correctly.\n",
        );
        for w in &self.words {
            s.push_str(&format!("- {}: {}\n", w.term, w.meaning));
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_and_matches() {
        let lx = parse("# c\nword variable: A name that holds a value. @ world.py: speed\nword game loop: Runs every frame.\nvague faster: Faster how?\n");
        assert_eq!(lx.words.len(), 2);
        assert_eq!(lx.words[0].place, "world.py: speed");
        assert!(mentions("Change the Variables please", "variable"));
        assert!(mentions("in the game loop, add", "game loop"));
        assert!(!mentions("invariable", "variable"));
        assert!(!mentions("variableness", "variable"));
        assert_eq!(lx.vague_in("make it faster").len(), 1);
        assert!(lx.vague_in("make it fast").is_empty());
    }
}
