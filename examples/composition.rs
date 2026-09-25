//! Run with: cargo run --example composition --features tokens
use htmlp::{Bindings, Cl100k, Document, Element};

fn main() {
    let greeting = Element::new("greeting")
        .id("welcome")
        .max_tokens(100, "Keep greetings brief.")
        .template("Hello, {{name}}!")
        .expect("valid template");
    let mut document = Document::new(500, "Shared context.", vec![greeting.into()]);
    document.sign(); // Explicitly accept the budgets authored above.
    let bindings = Bindings::from([("name".into(), "Alex".into())]);
    let counter = Cl100k::new().expect("tokenizer");
    let rendered = document.render(&bindings, &counter).expect("valid budgets");
    assert_eq!(rendered.element("welcome"), Some("Hello, Alex!"));
    assert_eq!(rendered.elements_by_name("greeting"), ["Hello, Alex!"]);
    println!("{}", rendered.text());
}
