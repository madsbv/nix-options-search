#![warn(clippy::all, clippy::pedantic)]
// #![warn(clippy::cargo)]
#![allow(
    clippy::missing_errors_doc,
    clippy::must_use_candidate,
    clippy::struct_excessive_bools,
    clippy::similar_names
)]

use anyhow::Result;

mod opt_data;

mod search;
#[allow(unused_imports)]
use search::{nix_darwin_searcher, nix_darwin_searcher_from_cache, search_for};

fn main() -> Result<()> {
    let mut nuc = nix_darwin_searcher_from_cache()?;
    let mut buffer = String::with_capacity(32);
    loop {
        buffer.clear();
        println!("Enter search query:");
        std::io::stdin().read_line(&mut buffer)?;

        let results = search_for(buffer.trim(), &mut nuc, 1000);
        eprintln!("{}", results.len());
        for m in results.iter().rev() {
            println!("{:#?}", m.data);
        }
        eprintln!("You searched for: {buffer}");
    }
}
