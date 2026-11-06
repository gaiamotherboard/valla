//! Items and where they live: one tab-separated file, rewritten whole on every change.

use std::path::{Path, PathBuf};

pub const STATUSES: [&str; 4] = ["in stock", "reserved", "sold", "retired"];

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub id: u64,
    pub name: String,
    pub status: String,
    pub location: String,
    pub updated: u64,
}

pub struct Store {
    path: PathBuf,
    pub items: Vec<Item>,
}

fn clean(s: &str) -> String {
    s.replace(['\t', '\n', '\r'], " ").trim().to_string()
}

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

impl Store {
    pub fn open(path: &Path) -> Store {
        let text = std::fs::read_to_string(path).unwrap_or_default();
        let items = text
            .lines()
            .filter_map(|l| {
                let f: Vec<&str> = l.split('\t').collect();
                if f.len() < 5 {
                    return None;
                }
                Some(Item { id: f[0].parse().ok()?, name: f[1].into(), status: f[2].into(), location: f[3].into(), updated: f[4].parse().unwrap_or(0) })
            })
            .collect();
        Store { path: path.to_path_buf(), items }
    }

    fn save(&self) -> std::io::Result<()> {
        let body: String = self.items.iter().map(|i| format!("{}\t{}\t{}\t{}\t{}\n", i.id, i.name, i.status, i.location, i.updated)).collect();
        let tmp = self.path.with_extension("tmp");
        std::fs::write(&tmp, body)?;
        std::fs::rename(&tmp, &self.path)
    }

    pub fn add(&mut self, name: &str, status: &str, location: &str) -> std::io::Result<u64> {
        let name = clean(name);
        if name.is_empty() {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "an item needs a name"));
        }
        let status = if STATUSES.contains(&status) { status.to_string() } else { STATUSES[0].to_string() };
        let id = self.items.iter().map(|i| i.id).max().unwrap_or(0) + 1;
        self.items.push(Item { id, name, status, location: clean(location), updated: now() });
        self.save()?;
        Ok(id)
    }

    pub fn set_status(&mut self, id: u64, status: &str) -> std::io::Result<bool> {
        if !STATUSES.contains(&status) {
            return Ok(false);
        }
        let Some(item) = self.items.iter_mut().find(|i| i.id == id) else { return Ok(false) };
        item.status = status.to_string();
        item.updated = now();
        self.save()?;
        Ok(true)
    }

    /// Items whose name, status or location contains every word of the query (any case).
    pub fn search(&self, q: &str) -> Vec<&Item> {
        let words: Vec<String> = q.split_whitespace().map(|w| w.to_lowercase()).collect();
        let mut v: Vec<&Item> = self
            .items
            .iter()
            .filter(|i| {
                let hay = format!("{} {} {}", i.name, i.status, i.location).to_lowercase();
                words.iter().all(|w| hay.contains(w))
            })
            .collect();
        v.sort_by(|a, b| b.updated.cmp(&a.updated).then(b.id.cmp(&a.id)));
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("inventarium-test-{}-{}.tsv", name, std::process::id()));
        let _ = std::fs::remove_file(&p);
        p
    }

    #[test]
    fn add_search_status_and_reload() {
        let p = temp("a");
        let mut s = Store::open(&p);
        let a = s.add("Dell OptiPlex 7050", "in stock", "Shelf A2").unwrap();
        s.add("ThinkPad X1\tCarbon", "nonsense", "Shelf B1").unwrap();
        assert_eq!(s.search("optiplex").len(), 1);
        assert_eq!(s.search("shelf").len(), 2);
        assert_eq!(s.items[1].name, "ThinkPad X1 Carbon");
        assert_eq!(s.items[1].status, "in stock");
        assert!(s.set_status(a, "sold").unwrap());
        assert!(!s.set_status(a, "stolen").unwrap());
        let s2 = Store::open(&p);
        assert_eq!(s2.items.len(), 2);
        assert_eq!(s2.search("sold")[0].id, a);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn empty_name_refused() {
        let p = temp("b");
        let mut s = Store::open(&p);
        assert!(s.add("   ", "in stock", "").is_err());
        let _ = std::fs::remove_file(&p);
    }
}
