#![warn(clippy::all, clippy::pedantic)]
// #![warn(clippy::cargo)]
#![allow(
    clippy::missing_errors_doc,
    clippy::must_use_candidate,
    clippy::struct_excessive_bools
)]

use anyhow::Result;
use tl::VDom;

mod opt_data;
use opt_data::{OptData, OptParser};

fn main() -> Result<()> {
    // let body: String = ureq::get("https://daiderd.com/nix-darwin/manual/index.html")
    //     .call()?
    //     .into_string()?;
    let body = std::fs::read_to_string("data/index-short.html").unwrap();
    let dom = tl::parse(&body, tl::ParserOptions::default())?;
    let opts = parse_options(&dom);
    // println!("{:#?}", opts.unwrap());

    for opt in opts.unwrap() {
        println!("{opt}");
    }
    Ok(())
}

// Structure of data/index.html (nix-darwin): Each option header is in a <dt>, associated description, type, default and link to docs is in a <dd>.
fn parse_options<'a>(dom: &'a VDom<'a>) -> Option<Vec<OptData<'a>>> {
    let p = dom.parser();
    // NodeHandles to all dt and dd tags, in order
    let varlist: Vec<_> = dom.query_selector("dt, dd")?.collect();

    // Entries of varlist should be pairs of dt followed by dd
    assert!(varlist.len() % 2 == 0);

    // Pair up dt and dd tags, parse and collect
    let mut opts = vec![];
    let mut index = 0;
    while index + 1 < varlist.len() {
        let parser = OptParser::new(*varlist.get(index)?, *varlist.get(index + 1)?, p);
        opts.push(parser.parse());
        index += 2;
    }
    Some(opts)
}
