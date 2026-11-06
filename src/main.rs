//! Valla, steward of the codebase.
//!
//! The kitchen (any model) cooks. Valla inspects, records, presents, and keeps the house
//! restorable. The owner tastes and decides. Nothing reaches the house without approval.

mod corrector;
mod form;
mod house;
mod journal;
mod json;
mod kitchen;
mod recipes;
mod sensors;
mod sha256;
mod util;
mod voice;

const HELP: &str = "Valla, steward of the codebase.

  valla init [recipe] [folder]   open a house (here, or lay out a recipe in a new folder)
  valla ask \"request\"            send a request to the kitchen; I inspect what returns
  valla table                    see what I present: what you asked, what I take it to mean,
                                 what nobody has said, what will change, what the sensors say
  valla diff                     the full change on the table
  valla approve                  serve it into the house (refused if the sensors object;
                                 valla approve --anyway overrules them, on the record)
  valla decline                  send it away
  valla restore [entry]          return the house to the last known-good state (or that entry)
  valla journal [count]          the books, and whether they are unbroken
  valla doctor                   machine, kitchen, journal and sensors, checked
  valla recipes                  sample tools I can lay out

Kitchen settings: ~/.config/valla/config  (kitchen_url, model, api_key)";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(|s| s.as_str()).unwrap_or("help");
    let rest = &args[1.min(args.len())..];
    let result = match cmd {
        "init" => house::init(rest),
        "ask" => {
            let req = rest.join(" ");
            if req.trim().is_empty() {
                Err("Ask me something. For example: valla ask \"add a notes field to items\"".into())
            } else {
                house::ask(&req)
            }
        }
        "table" => house::table(),
        "diff" => house::diff(),
        "approve" => house::approve(rest.iter().any(|a| a == "--anyway")),
        "decline" => house::decline(),
        "restore" => house::restore(rest.first().and_then(|s| s.parse().ok())),
        "journal" => house::journal(rest.first().and_then(|s| s.parse().ok()).unwrap_or(20)),
        "doctor" => house::doctor(),
        "recipes" => {
            house::list_recipes();
            Ok(())
        }
        "brag" => {
            voice::say("brag.immutable", &[]);
            Ok(())
        }
        "version" | "--version" => {
            println!("valla {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        _ => {
            println!("{}", HELP);
            Ok(())
        }
    };
    if let Err(e) = result {
        if !e.is_empty() {
            eprintln!("{}", e);
        }
        std::process::exit(1);
    }
}
