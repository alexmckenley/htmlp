fn main() {
    let source = std::env::args().nth(1).unwrap_or_else(|| {
        "<htmlp max-tokens=\"1k\" reason=\"Shared context.\"><task>Review.</task></htmlp>".into()
    });
    match htmlp::parse(&source) {
        Ok(doc) => println!("{doc}"),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
