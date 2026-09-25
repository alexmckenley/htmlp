//! Run with: cargo run --example runtime --features runtime,tokens
use htmlp::runtime::{
    ContentSource, ModelRequest, PromptFragment, ReportedUsage, Role, TokenUsage,
};
use htmlp::{Bindings, Cl100k, parse, render_checked};

fn main() {
    // This committed example is signed when its budgets are intentionally edited.
    let document = parse(include_str!("rules.htmlp")).expect("valid example");
    let counter = Cl100k::new().expect("tokenizer");
    let rendered = render_checked(&document, &Bindings::new(), &counter)
        .expect("valid signatures and budgets");
    let request = ModelRequest::builder("my-model")
        .fragment(PromptFragment::checked(
            ContentSource::SystemPrompt,
            Role::System,
            &rendered,
        ))
        .fragment(PromptFragment::text(
            ContentSource::UserMessage,
            Role::User,
            "Review the change.",
        ))
        .max_output_tokens(1024)
        .build();
    let usage = TokenUsage {
        estimate: Some(request.estimate()),
        // Example normalized provider totals, not a live model response.
        reported: Some(ReportedUsage {
            input_tokens: 120,
            cache_read_tokens: 80,
            output_tokens: 30,
            reasoning_tokens: 10,
            ..Default::default()
        }),
        context_window_tokens: 128_000,
    };
    assert_eq!(usage.prompt_tokens(), Some(200));
    println!("{}", serde_json::to_string_pretty(&usage).unwrap());
}
