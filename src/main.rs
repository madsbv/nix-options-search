use anyhow::Result;

fn main() -> Result<()> {
    let body: String = ureq::get("https://daiderd.com/nix-darwin/manual/index.html")
        .call()?
        .into_string()?;
    let dom = tl::parse(&body, tl::ParserOptions::default())?;
    println!("{body}");
    Ok(())
}

// Structure of data/index.html (nix-darwin): Each option header is in a <dt>, associated description, type, default and link to docs is in a <dd>. Format should be predictable.
//
// We could of course also try to parse from the man pages if we can grab them, would have to be kept up to date though.
