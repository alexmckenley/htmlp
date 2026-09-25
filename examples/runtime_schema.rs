fn main() {
    let schema = match std::env::args().nth(1).as_deref() {
        Some("request") => schemars::schema_for!(htmlp::runtime::ModelRequest),
        Some("usage") => schemars::schema_for!(htmlp::runtime::TokenUsage),
        _ => panic!("expected request or usage"),
    };
    println!("{}", serde_json::to_string_pretty(&schema).unwrap());
}
