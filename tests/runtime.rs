#![cfg(feature = "runtime")]
use htmlp::runtime::*;

#[test]
fn estimates_are_attributed_and_share_one_rounding_policy() {
    let request = ModelRequest::builder("test")
        .fragment(PromptFragment::text(
            ContentSource::SystemPrompt,
            Role::System,
            "a",
        ))
        .fragment(PromptFragment::text(
            ContentSource::UserMessage,
            Role::User,
            "b",
        ))
        .fragment(PromptFragment::text(
            ContentSource::UserMessage,
            Role::User,
            "cd",
        ))
        .build();
    let estimate = request.estimate();
    assert_eq!(estimate.total_tokens(), 2);
    assert_eq!(estimate.tokens_for(&ContentSource::UserMessage), 1);
    assert_eq!(
        estimate.sources().iter().map(|s| s.tokens).sum::<u64>(),
        estimate.total_tokens()
    );
    let usage = TokenUsage {
        estimate: Some(estimate),
        context_window_tokens: 1000,
        reported: None,
    };
    assert_eq!(usage.prompt_tokens(), Some(2));
}

#[test]
fn every_source_and_content_kind_is_counted_without_image_transport_inflation() {
    let sources = [
        ContentSource::SystemPrompt,
        ContentSource::ContextBlock {
            block_id: "rules".into(),
        },
        ContentSource::ToolDefinitions,
        ContentSource::UserMessage,
        ContentSource::SessionMessage,
        ContentSource::AssistantHistory,
        ContentSource::ToolOutput {
            tool_name: "shell".into(),
        },
        ContentSource::SystemReminder {
            origin: "mode".into(),
        },
        ContentSource::Hook {
            hook_name: "start".into(),
        },
        ContentSource::CompactionSummary,
    ];
    let mut builder = ModelRequest::builder("test");
    for source in &sources {
        builder = builder.fragment(PromptFragment::text(source.clone(), Role::User, "abcd"));
    }
    let request = builder
        .fragment(PromptFragment::new(
            ContentSource::UserMessage,
            Role::User,
            vec![
                ContentBlock::Reasoning {
                    text: "think".into(),
                },
                ContentBlock::Image {
                    media_type: "image/png".into(),
                    base64_data: "x".repeat(1_000_000),
                },
                ContentBlock::ToolUse {
                    tool_use_id: ToolUseId::from_string("a".into()),
                    tool_name: "shell".into(),
                    arguments: serde_json::json!({"cmd":"pwd"}),
                },
                ContentBlock::ToolResult {
                    tool_use_id: ToolUseId::from_string("a".into()),
                    content: "done".into(),
                    is_error: false,
                },
            ],
        ))
        .tool(ToolDefinition {
            name: "shell".into(),
            description: "Run".into(),
            input_schema: serde_json::json!({}),
        })
        .build();
    let estimate = request.estimate();
    assert_eq!(estimate.sources().len(), sources.len());
    assert!(sources.iter().all(|s| estimate.tokens_for(s) > 0));
    assert!(request.total_bytes() > 1_000_000);
    assert!(estimate.total_tokens() < 8300);
    assert_eq!(estimate.sources().iter().map(|s| s.images).sum::<u64>(), 1);
    let value = serde_json::to_value(&estimate).unwrap();
    assert_eq!(value["method"]["bytes_per_token"], 4);
    assert_eq!(value["method"]["image_tokens"], 8192);
    assert_eq!(
        serde_json::from_value::<RequestEstimate>(value).unwrap(),
        estimate
    );
}

#[test]
fn reported_only_usage_is_not_fake_category_attribution() {
    let reported = ReportedUsage {
        input_tokens: 10,
        cache_read_tokens: 20,
        cache_write_tokens: 3,
        output_tokens: 7,
        reasoning_tokens: 5,
    };
    let usage = TokenUsage {
        reported: Some(reported),
        estimate: None,
        context_window_tokens: 1000,
    };
    assert_eq!(usage.prompt_tokens(), Some(33));
    assert_eq!(
        serde_json::to_value(usage).unwrap()["estimate"],
        serde_json::Value::Null
    );
    assert_eq!(TokenUsage::default().prompt_tokens(), None);
}

#[test]
fn cumulative_snapshots_and_distinct_calls_have_different_operations() {
    let start = ReportedUsage {
        input_tokens: 12,
        output_tokens: 1,
        cache_read_tokens: 100,
        cache_write_tokens: 5,
        reasoning_tokens: 0,
    };
    let delta = ReportedUsage {
        output_tokens: 20,
        reasoning_tokens: 7,
        ..Default::default()
    };
    let mut merged = start.clone();
    merged.merge_snapshot(&delta);
    merged.merge_snapshot(&delta);
    assert_eq!(merged.prompt_tokens(), 117);
    assert_eq!((merged.output_tokens, merged.reasoning_tokens), (20, 7));
    let mut total = ReportedUsage::default();
    total.add_call(&merged);
    total.add_call(&merged);
    assert_eq!(total.prompt_tokens(), 234);
    assert_eq!((total.output_tokens, total.reasoning_tokens), (40, 14));
}

#[test]
fn checked_render_becomes_an_attributed_fragment_without_metadata_in_prompt() {
    let mut doc = htmlp::parse("<htmlp max-tokens='100' reason='Shared.'><section id='task'>Review {{subject}}.</section></htmlp>").unwrap();
    doc.sign();
    struct Count;
    impl htmlp::TokenCounter for Count {
        fn name(&self) -> &str {
            "cl100k_base"
        }
        fn count(&self, s: &str) -> u64 {
            s.len() as u64
        }
    }
    let result = htmlp::render_checked(
        &doc,
        &htmlp::Bindings::from([("subject".into(), "code".into())]),
        &Count,
    )
    .unwrap();
    let fragment = PromptFragment::checked(ContentSource::SystemPrompt, Role::System, &result);
    assert_eq!(fragment.source(), &ContentSource::SystemPrompt);
    assert_eq!(fragment.byte_count(), "Review code.".len());
    assert_eq!(
        fragment.content(),
        &[ContentBlock::Text {
            text: "Review code.".into()
        }]
    );
}
