fn main() {
    let schema = schemars::schema_for!(htmlp::Document);
    println!("{}", serde_json::to_string_pretty(&schema).unwrap());
}
