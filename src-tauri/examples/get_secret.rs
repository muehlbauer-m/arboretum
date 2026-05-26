use research_newsletter_lib::secrets;

fn main() {
    let name = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: cargo run --example get_secret -- <name>");
        std::process::exit(2);
    });
    match secrets::get(&name) {
        Ok(Some(v)) => print!("{v}"),
        Ok(None) => {
            eprintln!("(no entry for {name})");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}
