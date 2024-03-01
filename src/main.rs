#![warn(clippy::all, clippy::pedantic)]
// #![warn(clippy::cargo)]
#![allow(
    clippy::missing_errors_doc,
    clippy::must_use_candidate,
    clippy::struct_excessive_bools,
    clippy::similar_names
)]

use anyhow::Result;
use nucleo::pattern::{CaseMatching, Normalization};
use nucleo::{Config, Nucleo};

mod opt_data;
use opt_data::parse_options;

fn main() -> Result<()> {
    // let body: String = ureq::get("https://daiderd.com/nix-darwin/manual/index.html")
    //     .call()?
    //     .into_string()?;
    let body = std::fs::read_to_string("data/index.html").unwrap();
    // let body = std::fs::read_to_string("data/index-short.html").unwrap();
    let dom = tl::parse(&body, tl::ParserOptions::default())?;
    let opt_strings = parse_options(&dom).into_iter().map(|o| o.to_string());

    let mut nuc = create_nuc(1);
    let inj = nuc.injector();
    for s in opt_strings {
        // NOTE: First argument is the "data" part of matched items; use it to store the data you want to get out at the end (e.g. the entire object you're searching for, or an index to it).
        // The second argument is a closure that outputs the text that should be displayed as the user, and which Nucleo matches a given pattern against. For us, that could be the contents of the various fields of OptData in different columns
        inj.push(s.clone(), |fill| fill[0] = s.into());
    }
    nuc.tick(10);

    let mut buffer = String::with_capacity(32);
    loop {
        buffer.clear();
        println!("Enter search query:");
        std::io::stdin().read_line(&mut buffer)?;
        nuc.pattern.reparse(
            0,
            &buffer,
            CaseMatching::Ignore,
            Normalization::Smart,
            false,
        );

        // NOTE: tick(n) waits up to n ms for the matcher to finish, then returns. Here, we'd rather wait longer for it to really finish than risk it outputting the wrong thing.
        if nuc.tick(2000).running {
            println!("Searching is taking unusually long");
            while nuc.tick(1000).running {
                println!("working...");
            }
        }

        let snap = nuc.snapshot();
        let n = snap.matched_item_count();
        // NOTE: Can do `n.min(cap)` to output at most `cap` search results.
        for m in snap.matched_items(0..n).rev() {
            println!("{}", m.data);
        }
        eprintln!("You searched for: {buffer}");
        eprintln!("{n}");
    }
}

fn create_nuc(columns: u32) -> Nucleo<String> {
    Nucleo::<String>::new(
        Config::DEFAULT,
        std::sync::Arc::new(|| ()),
        Some(1),
        columns,
    )
}
