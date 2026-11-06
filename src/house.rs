//! The house: a git repository Valla keeps. Every duty of the steward lives here.

use crate::corrector;
use crate::form::{self, Change};
use crate::journal::Journal;
use crate::kitchen::{Kitchen, Message};
use crate::recipes;
use crate::sensors::{self, Reading};
use crate::util::{self, run};
use crate::voice::{line, say};
use std::path::{Path, PathBuf};

const KITCHEN_PROMPT: &str = include_str!("../prompts/kitchen.md");
const MAX_ATTEMPTS: usize = 3;
const FILE_LIMIT: usize = 40_000;
const CONTEXT_LIMIT: usize = 120_000;

fn git(dir: &Path, args: &[&str]) -> util::Out {
    let mut a = vec!["-c", "user.name=Valla", "-c", "user.email=valla@house.local", "-c", "commit.gpgsign=false"];
    a.extend_from_slice(args);
    run(dir, "git", &a)
}

pub fn find_root() -> Result<PathBuf, String> {
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let o = run(&cwd, "git", &["rev-parse", "--show-toplevel"]);
    if o.ok {
        Ok(PathBuf::from(o.text.trim()))
    } else {
        Err("This is not a house I keep. Run valla from inside a git repository, or start one with: valla init inventarium <folder>".into())
    }
}

fn setup(house: &Path) {
    let _ = std::fs::create_dir_all(house.join(".valla"));
    let ex = house.join(".git/info/exclude");
    let cur = util::read(&ex);
    if !cur.lines().any(|l| l.trim() == ".valla/") {
        util::write(&ex, &format!("{}{}.valla/\n", cur, if cur.is_empty() || cur.ends_with('\n') { "" } else { "\n" }));
    }
}

fn head(dir: &Path) -> String {
    git(dir, &["rev-parse", "HEAD"]).text.trim().to_string()
}

fn short(c: &str) -> String {
    c.chars().take(10).collect()
}

fn dirty(house: &Path) -> bool {
    !git(house, &["status", "--porcelain", "--untracked-files=no"]).text.trim().is_empty()
}

fn pending(house: &Path) -> Option<(String, PathBuf)> {
    let dir = house.join(".valla/pending");
    let mut ids: Vec<String> = std::fs::read_dir(&dir).ok()?.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().to_string()).collect();
    ids.sort();
    let id = ids.pop()?;
    let p = dir.join(&id);
    Some((id, p))
}

fn record_sensors(j: &Journal, house_label: &str, rs: &[Reading]) -> u64 {
    let report = sensors::report(rs);
    j.write("reading", &[("where", house_label), ("summary", &sensors::summary(rs)), ("report", &report)])
}

// ---------- init ----------

pub fn init(args: &[String]) -> Result<(), String> {
    match args.first() {
        None => {
            let house = find_root()?;
            setup(&house);
            let j = Journal::open(&house);
            if !j.entries().is_empty() {
                say("journal.ok", &[("count", &j.entries().len().to_string())]);
                return Ok(());
            }
            let rs = sensors::run_all(&house);
            record_sensors(&j, "house", &rs);
            let good = if sensors::all_pass(&rs) { "yes" } else { "no" };
            let n = j.write("init", &[("commit", &head(&house)), ("known_good", good), ("sensors", &sensors::summary(&rs))]);
            say("greet.init", &[("n", &n.to_string())]);
            if rs.is_empty() {
                println!("No sensors are declared. Add a valla.sensors file (name: command, one per line) so I have something to swear by.");
            }
            Ok(())
        }
        Some(name) => {
            let recipe = recipes::find(name).ok_or_else(|| format!("I know no recipe called {}. valla recipes lists them.", name))?;
            let dir = PathBuf::from(args.get(1).cloned().unwrap_or_else(|| recipe.name.to_string()));
            if dir.exists() && std::fs::read_dir(&dir).map(|mut d| d.next().is_some()).unwrap_or(false) {
                return Err(format!("{} already has things in it. I lay out recipes only in empty rooms.", dir.display()));
            }
            for (p, c) in recipe.files {
                util::write(&dir.join(p), c);
            }
            let dir = std::fs::canonicalize(&dir).map_err(|e| e.to_string())?;
            git(&dir, &["init", "-q", "-b", "main"]);
            // Nix only sees files git knows about, so stage everything before pinning.
            git(&dir, &["add", "-A"]);
            if dir.join("flake.nix").exists() && std::env::var("VALLA_NO_NIX").is_err() && run(&dir, "nix", &["--version"]).ok {
                println!("Pinning the instruments (nix flake lock). The first time takes a minute.");
                let o = run(&dir, "nix", &["flake", "lock"]);
                if !o.ok {
                    println!("Nix could not pin the flake: {}", o.text);
                }
            }
            git(&dir, &["add", "-A"]);
            let c = git(&dir, &["commit", "-q", "-m", &format!("valla: lay out the {} recipe", recipe.name)]);
            if !c.ok {
                return Err(format!("git commit failed: {}", c.text));
            }
            setup(&dir);
            let j = Journal::open(&dir);
            println!("Consulting the sensors. The first build takes a while.");
            let rs = sensors::run_all(&dir);
            record_sensors(&j, "house", &rs);
            let good = if sensors::all_pass(&rs) { "yes" } else { "no" };
            j.write("init", &[("recipe", recipe.name), ("commit", &head(&dir)), ("known_good", good), ("sensors", &sensors::summary(&rs))]);
            say("greet.init_recipe", &[("recipe", recipe.name), ("dir", &dir.display().to_string()), ("result", &sensors::summary(&rs))]);
            if !rs.iter().all(|r| r.pass) {
                println!("{}", sensors::report(&rs));
            }
            println!("Next:  cd {}  then  valla ask \"what you would like\"", dir.display());
            Ok(())
        }
    }
}

// ---------- ask ----------

fn context(house: &Path) -> String {
    let files = git(house, &["ls-files"]).text;
    let mut out = String::new();
    let mut skipped = Vec::new();
    for f in files.lines().filter(|l| !l.is_empty()) {
        if f == "Cargo.lock" || f == "flake.lock" {
            skipped.push(f.to_string());
            continue;
        }
        let o = git(house, &["show", &format!("HEAD:{}", f)]);
        if !o.ok || o.text.contains('\0') || o.text.len() > FILE_LIMIT || out.len() + o.text.len() > CONTEXT_LIMIT {
            skipped.push(f.to_string());
            continue;
        }
        out.push_str(&format!("=== FILE {} ===\n{}\n=== END {} ===\n\n", f, o.text, f));
    }
    if !skipped.is_empty() {
        out.push_str(&format!("(Not shown, too large or not text: {})\n", skipped.join(", ")));
    }
    out
}

pub fn ask(request: &str) -> Result<(), String> {
    let house = find_root()?;
    setup(&house);
    if let Some((id, _)) = pending(&house) {
        say("ask.pending", &[("id", &id)]);
        return Ok(());
    }
    if dirty(&house) {
        say("ask.dirty", &[]);
    }
    let j = Journal::open(&house);
    let base = head(&house);
    let n = j.write("request", &[("text", request), ("base", &base)]);
    let id = format!("{:04}", n);
    say("ask.received", &[("n", &n.to_string())]);

    let kitchen = Kitchen::from_settings();
    let work = house.join(".valla/work").join(&id);
    let _ = std::fs::create_dir_all(&work);
    j.write("brief", &[("request", &n.to_string()), ("kitchen", &kitchen.describe())]);

    let mut msgs = vec![
        Message { role: "system", content: KITCHEN_PROMPT.to_string() },
        Message { role: "user", content: format!("REQUEST:\n{}\n\nFILES (committed state {}):\n\n{}", request, short(&base), context(&house)) },
    ];
    let mut accepted: Option<(form::Form, String)> = None;
    let mut first_reject = 0;
    let mut last_reject = 0;
    for attempt in 1..=MAX_ATTEMPTS {
        println!("The kitchen is at work (attempt {} of {}, {}).", attempt, MAX_ATTEMPTS, kitchen.describe());
        let (raw, secs) = match kitchen.cook(&msgs, attempt, &work) {
            Ok(x) => x,
            Err(e) => {
                j.write("kitchen_down", &[("request", &n.to_string()), ("detail", &e)]);
                say("ask.kitchen_down", &[("detail", &e)]);
                return Ok(());
            }
        };
        let file = work.join(format!("return-{}.md", attempt));
        util::write(&file, &raw);
        j.write("return", &[("request", &n.to_string()), ("attempt", &attempt.to_string()), ("sha256", &crate::sha256::hex(raw.as_bytes())), ("seconds", &format!("{:.0}", secs))]);
        say("ask.returned", &[("attempt", &attempt.to_string())]);
        let f = form::parse(&raw);
        let faults = corrector::inspect(&f, request, &house);
        if faults.is_empty() {
            j.write("inspection", &[("request", &n.to_string()), ("attempt", &attempt.to_string()), ("result", "pass")]);
            accepted = Some((f, raw));
            break;
        }
        let text: Vec<String> = faults.iter().map(|x| x.message.clone()).collect();
        let rules: Vec<&str> = faults.iter().map(|x| x.rule).collect();
        let r = j.write("rejection", &[("request", &n.to_string()), ("attempt", &attempt.to_string()), ("rules", &rules.join(",")), ("faults", &text.join("\n"))]);
        if first_reject == 0 {
            first_reject = r;
        }
        last_reject = r;
        say("ask.rejected", &[("count", &faults.len().to_string())]);
        for t in &text {
            println!("  - {}", t);
        }
        msgs.push(Message { role: "assistant", content: raw });
        msgs.push(Message { role: "user", content: format!("{}\n{}", line("feedback.header", &[]), text.iter().map(|t| format!("- {}", t)).collect::<Vec<_>>().join("\n")) });
    }
    let (f, raw) = match accepted {
        Some(x) => x,
        None => {
            j.write("escalation", &[("request", &n.to_string())]);
            say("ask.escalate", &[("attempts", &MAX_ATTEMPTS.to_string()), ("from", &first_reject.to_string()), ("to", &last_reject.to_string())]);
            return Ok(());
        }
    };

    // Into the playground: a separate worktree on its own branch, from the committed state.
    let branch = format!("valla/{}", id);
    let pg = house.join(".valla/playground").join(&id);
    let o = git(&house, &["worktree", "add", "-q", "-b", &branch, &pg.display().to_string(), &base]);
    if !o.ok {
        return Err(format!("could not open the playground: {}", o.text));
    }
    for ch in &f.changes {
        match ch {
            Change::Write { path, content } => util::write(&pg.join(path), content),
            Change::Delete { path } => {
                let _ = std::fs::remove_file(pg.join(path));
            }
        }
    }
    git(&pg, &["add", "-A"]);
    // The observed diff, not the claimed one, must trace back to the plan.
    let changed: Vec<String> = git(&pg, &["diff", "--cached", "--name-only"]).text.lines().map(String::from).filter(|l| !l.is_empty()).collect();
    let plan_text: String = f.plan.iter().map(|p| p.text.clone()).collect::<Vec<_>>().join("\n");
    let untraced: Vec<&String> = changed.iter().filter(|c| !plan_text.contains(c.as_str())).collect();
    if !untraced.is_empty() || changed.is_empty() {
        let why = if changed.is_empty() { "the changes alter nothing".to_string() } else { format!("eisegesis in the observed diff: {}", untraced.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")) };
        j.write("escalation", &[("request", &n.to_string()), ("detail", &why)]);
        git(&house, &["worktree", "remove", "--force", &pg.display().to_string()]);
        git(&house, &["branch", "-D", &branch]);
        println!("I will not serve this: {}.", why);
        return Ok(());
    }
    let first_line = request.lines().next().unwrap_or("").chars().take(60).collect::<String>();
    let c = git(&pg, &["commit", "-q", "-m", &format!("valla {}: {}", id, first_line)]);
    if !c.ok {
        return Err(format!("commit in the playground failed: {}", c.text));
    }
    let proposal = head(&pg);
    println!("Consulting the sensors in the playground.");
    let rs = sensors::run_all(&pg);
    record_sensors(&j, &format!("playground {}", id), &rs);

    let pd = house.join(".valla/pending").join(&id);
    util::write(&pd.join("request.txt"), request);
    util::write(&pd.join("return.md"), &raw);
    util::write(&pd.join("sensors.txt"), &format!("{}\n{}", sensors::summary(&rs), sensors::report(&rs)));
    util::write(&pd.join("meta"), &format!("entry={}\nbranch={}\ncommit={}\nbase={}\nwhen={}\n", n, branch, proposal, base, util::iso(util::now_secs())));
    j.write("presentation", &[("request", &n.to_string()), ("commit", &proposal), ("sensors", &sensors::summary(&rs))]);
    say("ask.served", &[("id", &id), ("result", &sensors::summary(&rs))]);
    Ok(())
}

fn meta(pd: &Path, key: &str) -> String {
    util::read(&pd.join("meta")).lines().find_map(|l| l.strip_prefix(&format!("{}=", key)).map(String::from)).unwrap_or_default()
}

// ---------- table, diff ----------

pub fn table() -> Result<(), String> {
    let house = find_root()?;
    let Some((id, pd)) = pending(&house) else {
        say("table.empty", &[]);
        return Ok(());
    };
    let f = form::parse(&util::read(&pd.join("return.md")));
    let rule = "-".repeat(60);
    say("table.header", &[("id", &id), ("when", &meta(&pd, "when"))]);
    println!("\n{}\n{}", line("table.asked", &[]), rule);
    println!("{}", util::read(&pd.join("request.txt")).trim());
    println!("\n{}\n{}", line("table.meaning", &[]), rule);
    println!("On the record:");
    for q in &f.exegesis {
        println!("  {}  \"{}\"  ({})", q.id, q.text, q.source);
    }
    println!("My reading:");
    for h in &f.hermeneutics {
        println!("  {} [{}]  {}", h.id, h.cites.join(","), h.text);
    }
    println!("\n{}\n{}", line("table.silence", &[]), rule);
    if f.silence.is_empty() {
        println!("{}", line("table.silence_none", &[]));
    }
    for s in &f.silence {
        println!("  - {}", s);
    }
    println!("\n{}\n{}", line("table.change", &[]), rule);
    for p in &f.plan {
        println!("  {} [{}]  {}", p.id, p.cites.join(","), p.text);
    }
    println!("{}", git(&house, &["diff", "--stat", &meta(&pd, "base"), &meta(&pd, "commit")]).text);
    println!("\n{}\n{}", line("table.sensors", &[]), rule);
    println!("{}", util::read(&pd.join("sensors.txt")).trim());
    println!("\n{}", line("table.footer", &[]));
    Ok(())
}

pub fn diff() -> Result<(), String> {
    let house = find_root()?;
    let Some((_, pd)) = pending(&house) else {
        say("table.empty", &[]);
        return Ok(());
    };
    println!("{}", git(&house, &["diff", &meta(&pd, "base"), &meta(&pd, "commit")]).text);
    Ok(())
}

fn clear(house: &Path, id: &str, pd: &Path) {
    let pg = house.join(".valla/playground").join(id);
    git(house, &["worktree", "remove", "--force", &pg.display().to_string()]);
    git(house, &["worktree", "prune"]);
    git(house, &["branch", "-D", &format!("valla/{}", id)]);
    let _ = std::fs::remove_dir_all(pd);
}

// ---------- approve, decline ----------

pub fn approve(anyway: bool) -> Result<(), String> {
    let house = find_root()?;
    let Some((id, pd)) = pending(&house) else {
        say("approve.none", &[]);
        return Ok(());
    };
    let playground_passed = util::read(&pd.join("sensors.txt")).lines().next().map(|l| l.contains("pass") && !l.contains("FAIL")).unwrap_or(false);
    if !playground_passed && !anyway {
        say("approve.sensors_object", &[]);
        return Ok(());
    }
    if !playground_passed {
        Journal::open(&house).write("override", &[("proposal", &id), ("detail", "owner approved over the sensors' objection")]);
    }
    if dirty(&house) {
        say("approve.blocked", &[("detail", "the house has uncommitted changes of its own. Commit or set them aside first.")]);
        return Ok(());
    }
    let commit = meta(&pd, "commit");
    let m = git(&house, &["merge", "--ff-only", "-q", &commit]);
    if !m.ok {
        say("approve.blocked", &[("detail", &format!("the house has moved since this was prepared ({}). Decline it and ask again.", m.text.lines().next().unwrap_or("")))]);
        return Ok(());
    }
    clear(&house, &id, &pd);
    let j = Journal::open(&house);
    let prior_good = j.last_known_good().map(|e| e.n.to_string()).unwrap_or_else(|| "none".into());
    println!("Consulting the sensors in the house.");
    let rs = sensors::run_all(&house);
    record_sensors(&j, "house", &rs);
    let good = sensors::all_pass(&rs);
    let n = j.write(
        "approval",
        &[("proposal", &id), ("commit", &head(&house)), ("sensors", &sensors::summary(&rs)), ("known_good", if good { "yes" } else { "no" })],
    );
    say("approve.done", &[("n", &n.to_string()), ("commit", &short(&head(&house))), ("result", &sensors::summary(&rs))]);
    if good {
        say("approve.known_good", &[]);
    } else {
        println!("{}", sensors::report(&rs));
        say("approve.sensors_failed", &[("good", &prior_good)]);
    }
    Ok(())
}

pub fn decline() -> Result<(), String> {
    let house = find_root()?;
    let Some((id, pd)) = pending(&house) else {
        say("table.empty", &[]);
        return Ok(());
    };
    let commit = meta(&pd, "commit");
    clear(&house, &id, &pd);
    let n = Journal::open(&house).write("decline", &[("proposal", &id), ("commit", &commit)]);
    say("decline.done", &[("n", &n.to_string())]);
    Ok(())
}

// ---------- restore ----------

pub fn restore(target: Option<u64>) -> Result<(), String> {
    let house = find_root()?;
    let j = Journal::open(&house);
    let entry = match target {
        Some(n) => j.get(n).filter(|e| e.data.get("known_good").map(|s| s == "yes").unwrap_or(false)),
        None => j.last_known_good(),
    };
    let Some(e) = entry else {
        say("restore.none", &[]);
        return Ok(());
    };
    let commit = e.data.get("commit").cloned().unwrap_or_default();
    if dirty(&house) {
        say("approve.blocked", &[("detail", "the house has uncommitted changes of its own. Commit or set them aside first.")]);
        return Ok(());
    }
    let same = git(&house, &["diff", "--quiet", &commit, "HEAD"]).ok;
    if same {
        say("restore.same", &[("n", &e.n.to_string())]);
        return Ok(());
    }
    let r = git(&house, &["restore", &format!("--source={}", commit), "--staged", "--worktree", "--", ":/"]);
    if !r.ok {
        return Err(format!("restore failed: {}", r.text));
    }
    let c = git(&house, &["commit", "-q", "-m", &format!("valla: restore to journal entry {} ({})", e.n, short(&commit))]);
    if !c.ok {
        return Err(format!("commit of restoration failed: {}", c.text));
    }
    println!("Consulting the sensors in the house.");
    let rs = sensors::run_all(&house);
    record_sensors(&j, "house", &rs);
    let good = sensors::all_pass(&rs);
    let n = j.write(
        "restore",
        &[("to_entry", &e.n.to_string()), ("source_commit", &commit), ("commit", &head(&house)), ("sensors", &sensors::summary(&rs)), ("known_good", if good { "yes" } else { "no" })],
    );
    say("restore.done", &[("n", &e.n.to_string()), ("commit", &short(&commit)), ("entry", &n.to_string()), ("result", &sensors::summary(&rs))]);
    Ok(())
}

// ---------- journal, doctor, recipes ----------

pub fn journal(count: usize) -> Result<(), String> {
    let house = find_root()?;
    let j = Journal::open(&house);
    let all = j.entries();
    let skip = all.len().saturating_sub(count);
    for e in &all[skip..] {
        let mut parts: Vec<String> = Vec::new();
        for (k, v) in &e.data {
            if k == "report" || k == "faults" {
                continue;
            }
            let v1: String = v.lines().next().unwrap_or("").chars().take(60).collect();
            parts.push(format!("{}={}", k, v1));
        }
        println!("{:>4}  {}  {:<13} {}", e.n, e.ts, e.kind, parts.join("  "));
    }
    match j.verify() {
        Ok(c) => say("journal.ok", &[("count", &c.to_string())]),
        Err(n) => say("journal.broken", &[("n", &n.to_string())]),
    }
    Ok(())
}

pub fn doctor() -> Result<(), String> {
    let here = std::env::current_dir().map_err(|e| e.to_string())?;
    let house = find_root().ok();
    let mut faults = 0;
    let readings = sensors::machine(&here);
    let host = readings.iter().find(|(k, _)| k == "host").map(|(_, v)| v.clone()).unwrap_or_default();
    say("doctor.header", &[("host", &host)]);
    for (k, v) in &readings {
        println!("  {:<14} {}", k, v);
    }
    for tool in ["git", "curl", "nix"] {
        let o = run(&here, tool, &["--version"]);
        let first = o.text.lines().next().unwrap_or("").to_string();
        println!("  {:<14} {}", tool, if o.ok { first } else { "MISSING".into() });
        if !o.ok && tool != "nix" {
            faults += 1;
        }
    }
    let kitchen = Kitchen::from_settings();
    let scratch = std::env::temp_dir();
    let speed = match kitchen.ping(&scratch) {
        Ok(s) => {
            println!("  {:<14} {} {}", "kitchen", kitchen.describe(), s);
            s
        }
        Err(e) => {
            faults += 1;
            println!("  {:<14} {} NOT ANSWERING: {}", "kitchen", kitchen.describe(), e.lines().next().unwrap_or(""));
            "silent".into()
        }
    };
    if let Some(h) = &house {
        let j = Journal::open(h);
        match j.verify() {
            Ok(c) => println!("  {:<14} {} entries, unbroken", "journal", c),
            Err(n) => {
                faults += 1;
                say("journal.broken", &[("n", &n.to_string())]);
            }
        }
        let good = j.last_known_good().map(|e| format!("entry {} ({})", e.n, short(e.data.get("commit").map(|s| s.as_str()).unwrap_or("")))).unwrap_or_else(|| "none yet".into());
        println!("  {:<14} {}", "known good", good);
        let rs = sensors::run_all(h);
        println!("  {:<14} {}", "sensors", sensors::summary(&rs));
        if !rs.is_empty() {
            println!("{}", sensors::report(&rs));
        }
        if !rs.iter().all(|r| r.pass) {
            faults += 1;
        }
        if j.verify().is_ok() {
            let report = readings.iter().map(|(k, v)| format!("{}: {}", k, v)).collect::<Vec<_>>().join("\n");
            j.write("doctor", &[("machine", &report), ("kitchen", &speed), ("sensors", &sensors::summary(&rs))]);
        }
    }
    if faults == 0 {
        say("doctor.ok", &[("speed", &speed)]);
        Ok(())
    } else {
        say("doctor.faults", &[("count", &faults.to_string())]);
        Err(String::new())
    }
}

pub fn list_recipes() {
    for r in recipes::ALL {
        println!("  {:<14} {}", r.name, r.about);
    }
    println!("\nLay one out with: valla init <recipe> <folder>");
}
