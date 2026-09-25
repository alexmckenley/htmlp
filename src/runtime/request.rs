//! Compile-time-enforced content attribution.
//!
//! Everything that reaches the model flows through [`PromptFragment`], which is
//! only constructible with a [`ContentSource`] tag. [`ModelRequest`] is
//! assembled exclusively from fragments, so per-request byte attribution by
//! category is always available — there is no untagged path to the LLM.

use std::collections::BTreeMap;

use serde::Deserialize;
use serde::Serialize;

use super::{ByteEstimator, RequestEstimate, Role, ToolUseId};

/// The origin category of a piece of model-bound content.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ContentSource {
    SystemPrompt,
    /// A typed context block (env details, rules, skills catalog, ...).
    ContextBlock {
        block_id: String,
    },
    ToolDefinitions,
    UserMessage,
    SessionMessage,
    /// Prior assistant output replayed as history.
    AssistantHistory,
    ToolOutput {
        tool_name: String,
    },
    /// Harness-injected content, distinct from user-authored messages.
    SystemReminder {
        origin: String,
    },
    Hook {
        hook_name: String,
    },
    CompactionSummary,
}

/// Provider-neutral content block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text {
        text: String,
    },
    /// Provider reasoning returned alongside an assistant message. Adapters
    /// that support preserved thinking replay this through their native
    /// field; other adapters deliberately omit it.
    Reasoning {
        text: String,
    },
    Image {
        media_type: String,
        base64_data: String,
    },
    ToolUse {
        tool_use_id: ToolUseId,
        tool_name: String,
        arguments: serde_json::Value,
    },
    ToolResult {
        tool_use_id: ToolUseId,
        content: String,
        is_error: bool,
    },
}

impl ContentBlock {
    pub fn byte_len(&self) -> usize {
        match self {
            ContentBlock::Text { text } => text.len(),
            ContentBlock::Reasoning { text } => text.len(),
            ContentBlock::Image { base64_data, .. } => base64_data.len(),
            ContentBlock::ToolUse { arguments, .. } => arguments.to_string().len(),
            ContentBlock::ToolResult { content, .. } => content.len(),
        }
    }
}

/// A source-attributed piece of a model request. Fields are private; the only
/// constructors require attribution at the call site.
///
/// ```compile_fail
/// use htmlp::runtime::{PromptFragment, Role};
/// let fragment = PromptFragment::text(Role::User, "untagged");
/// ```
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PromptFragment {
    source: ContentSource,
    role: Role,
    content: Vec<ContentBlock>,
    byte_count: usize,
}

impl PromptFragment {
    pub fn new(source: ContentSource, role: Role, content: Vec<ContentBlock>) -> Self {
        let byte_count = content.iter().map(ContentBlock::byte_len).sum();
        Self {
            source,
            role,
            content,
            byte_count,
        }
    }

    pub fn text(source: ContentSource, role: Role, text: impl Into<String>) -> Self {
        Self::new(source, role, vec![ContentBlock::Text { text: text.into() }])
    }

    /// Construct a text fragment from immutable, fully checked HTMLP output.
    pub fn checked(source: ContentSource, role: Role, rendered: &crate::RenderedPrompt) -> Self {
        Self::text(source, role, rendered.text())
    }

    pub fn source(&self) -> &ContentSource {
        &self.source
    }

    pub fn role(&self) -> Role {
        self.role
    }

    pub fn content(&self) -> &[ContentBlock] {
        &self.content
    }

    pub fn byte_count(&self) -> usize {
        self.byte_count
    }
}

/// A tool definition offered to the model. Counted under
/// [`ContentSource::ToolDefinitions`] in attribution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

impl ToolDefinition {
    pub fn byte_len(&self) -> usize {
        self.name.len() + self.description.len() + self.input_schema.to_string().len()
    }
}

/// Reasoning/thinking effort. Providers map this to their native knob
/// (Anthropic `budget_tokens`, OpenAI `reasoning_effort`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum ThinkingLevel {
    Off,
    Low,
    Medium,
    #[default]
    High,
    /// Serialized as "xhigh". Providers clamp to the model's max effort.
    XHigh,
    /// Provider-native maximum reasoning effort when distinct from xhigh.
    Max,
}

impl ThinkingLevel {
    /// The canonical lowercase name, matching the serde representation.
    /// Provider adapters that need a different word for a level (codex
    /// spells `Off` as "none") map it themselves.
    pub fn as_str(&self) -> &'static str {
        match self {
            ThinkingLevel::Off => "off",
            ThinkingLevel::Low => "low",
            ThinkingLevel::Medium => "medium",
            ThinkingLevel::High => "high",
            ThinkingLevel::XHigh => "xhigh",
            ThinkingLevel::Max => "max",
        }
    }
}

impl std::fmt::Display for ThinkingLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A complete, attributed model request. Constructed only via [`ModelRequestBuilder`].
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ModelRequest {
    fragments: Vec<PromptFragment>,
    tools: Vec<ToolDefinition>,
    pub model: String,
    pub max_output_tokens: Option<u32>,
    pub thinking_level: ThinkingLevel,
}

impl ModelRequest {
    pub fn builder(model: impl Into<String>) -> ModelRequestBuilder {
        ModelRequestBuilder {
            fragments: Vec::new(),
            tools: Vec::new(),
            model: model.into(),
            max_output_tokens: None,
            thinking_level: ThinkingLevel::default(),
        }
    }

    pub fn fragments(&self) -> &[PromptFragment] {
        &self.fragments
    }

    pub fn tools(&self) -> &[ToolDefinition] {
        &self.tools
    }

    /// Byte attribution by source category, including tool definitions.
    /// The heart of the attribution invariant: this is derivable for every
    /// request because no content can bypass fragment construction.
    pub fn bytes_by_source(&self) -> BTreeMap<ContentSource, usize> {
        let mut result: BTreeMap<ContentSource, usize> = BTreeMap::new();
        for fragment in &self.fragments {
            *result.entry(fragment.source.clone()).or_default() += fragment.byte_count;
        }
        let tool_bytes: usize = self.tools.iter().map(ToolDefinition::byte_len).sum();
        if tool_bytes > 0 {
            *result.entry(ContentSource::ToolDefinitions).or_default() += tool_bytes;
        }
        result
    }

    /// Categorized heuristic estimate. Tokenizer counts and provider-reported
    /// usage are separate measurements; this does not count wire framing.
    pub fn estimate(&self) -> RequestEstimate {
        ByteEstimator::default().estimate(self)
    }

    pub fn total_bytes(&self) -> usize {
        self.bytes_by_source().values().sum()
    }

    /// Prefix-cache pattern: the live request, unchanged, with one
    /// instruction appended as a suffix (compaction summaries, titles).
    /// Tool definitions are dropped — suffix requests want plain text back.
    pub fn with_suffix_instruction(&self, source: ContentSource, instruction: &str) -> Self {
        let mut request = self.clone();
        request.tools = Vec::new();
        request
            .fragments
            .push(PromptFragment::text(source, Role::User, instruction));
        request
    }

    /// Cache-warm probe: the request unchanged — tool definitions kept,
    /// they are part of the provider's cached prefix (Anthropic caches
    /// tools → system → messages) — plus one throwaway user fragment
    /// (providers reject empty message lists). The fragment sits after the
    /// system-block cache breakpoint, so its content never affects the
    /// cached prefix. `max_output_tokens: 1` caps waste if the caller's
    /// first-event abort is slow.
    pub fn with_warm_probe(&self) -> Self {
        let mut request = self.clone();
        request.max_output_tokens = Some(1);
        request.fragments.push(PromptFragment::text(
            ContentSource::SystemReminder {
                origin: "cache-warm".to_string(),
            },
            Role::User,
            "ping",
        ));
        request
    }

    /// The request with every image block replaced by a text note, for
    /// models whose endpoint accepts text content only — sending the
    /// image block would fail the whole request at the provider. The
    /// session log is untouched: history keeps the real image parts, so
    /// switching to an image-capable model replays them as images again.
    /// Attribution is preserved per fragment; only the image bytes become
    /// the note's text.
    pub fn without_image_blocks(&self) -> Self {
        let mut request = self.clone();
        for fragment in &mut request.fragments {
            if !fragment
                .content()
                .iter()
                .any(|block| matches!(block, ContentBlock::Image { .. }))
            {
                continue;
            }
            let replaced = fragment
                .content()
                .iter()
                .map(|block| match block {
                    ContentBlock::Image { .. } => ContentBlock::Text {
                        text: TEXT_ONLY_IMAGE_NOTE.to_string(),
                    },
                    other => other.clone(),
                })
                .collect();
            *fragment = PromptFragment::new(fragment.source().clone(), fragment.role(), replaced);
        }
        request
    }
}

/// Shown to the model in place of an image the active model cannot view.
/// Reads as a bracketed aside so tool-result descriptions ("Image foo.png:
/// 800×600 …") still read as the metadata line it accompanies.
pub const TEXT_ONLY_IMAGE_NOTE: &str = "[image not shown: this model does not support image input]";

pub struct ModelRequestBuilder {
    fragments: Vec<PromptFragment>,
    tools: Vec<ToolDefinition>,
    model: String,
    max_output_tokens: Option<u32>,
    thinking_level: ThinkingLevel,
}

impl ModelRequestBuilder {
    /// The only way to add content — a pre-attributed fragment.
    pub fn fragment(mut self, fragment: PromptFragment) -> Self {
        self.fragments.push(fragment);
        self
    }

    pub fn tool(mut self, tool: ToolDefinition) -> Self {
        self.tools.push(tool);
        self
    }

    pub fn max_output_tokens(mut self, tokens: u32) -> Self {
        self.max_output_tokens = Some(tokens);
        self
    }

    pub fn thinking_level(mut self, level: ThinkingLevel) -> Self {
        self.thinking_level = level;
        self
    }

    pub fn build(self) -> ModelRequest {
        ModelRequest {
            fragments: self.fragments,
            tools: self.tools,
            model: self.model,
            max_output_tokens: self.max_output_tokens,
            thinking_level: self.thinking_level,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_transport_bytes_do_not_trigger_text_sized_compaction() {
        let request = ModelRequest::builder("vision")
            .fragment(PromptFragment::new(
                ContentSource::ToolOutput {
                    tool_name: "read_file".into(),
                },
                Role::User,
                vec![ContentBlock::Image {
                    media_type: "image/jpeg".into(),
                    base64_data: "A".repeat(1_300_000),
                }],
            ))
            .build();
        assert_eq!(
            request.total_bytes(),
            1_300_000,
            "raw byte attribution is exact"
        );
        assert_eq!(request.estimate().total_tokens(), 8192);
    }

    #[test]
    fn bytes_by_source_attributes_every_category() {
        let request = ModelRequest::builder("test-model")
            .fragment(PromptFragment::text(
                ContentSource::SystemPrompt,
                Role::System,
                "base",
            ))
            .fragment(PromptFragment::text(
                ContentSource::ContextBlock {
                    block_id: "workspace".to_string(),
                },
                Role::System,
                "cwd:/repo",
            ))
            .fragment(PromptFragment::text(
                ContentSource::UserMessage,
                Role::User,
                "hello!",
            ))
            .tool(ToolDefinition {
                name: "bash".to_string(),
                description: "run".to_string(),
                input_schema: serde_json::json!({}),
            })
            .build();

        let bytes = request.bytes_by_source();
        assert_eq!(
            bytes[&ContentSource::SystemPrompt],
            4,
            "system prompt bytes = len('base')"
        );
        assert_eq!(
            bytes[&ContentSource::ContextBlock {
                block_id: "workspace".to_string()
            }],
            9,
            "context block bytes = len('cwd:/repo')"
        );
        assert_eq!(
            bytes[&ContentSource::UserMessage],
            6,
            "user bytes = len('hello!')"
        );
        assert!(
            bytes[&ContentSource::ToolDefinitions] >= 9,
            "tool defs counted (name+description+schema)"
        );
        assert_eq!(
            bytes.values().sum::<usize>(),
            request.total_bytes(),
            "totals agree"
        );
    }

    #[test]
    fn without_image_blocks_swaps_notes_and_keeps_attribution() {
        let request = ModelRequest::builder("zai/glm-5.3")
            .fragment(PromptFragment::new(
                ContentSource::UserMessage,
                Role::User,
                vec![
                    ContentBlock::Text {
                        text: "what is this?".into(),
                    },
                    ContentBlock::Image {
                        media_type: "image/png".into(),
                        base64_data: "aGVsbG8=".into(),
                    },
                ],
            ))
            .fragment(PromptFragment::text(
                ContentSource::SystemPrompt,
                Role::System,
                "sys",
            ))
            .build();

        let replaced = request.without_image_blocks();
        let blocks = replaced.fragments()[0].content();
        assert_eq!(
            blocks[0],
            ContentBlock::Text {
                text: "what is this?".into()
            },
            "neighboring text blocks pass through"
        );
        assert_eq!(
            blocks[1],
            ContentBlock::Text {
                text: TEXT_ONLY_IMAGE_NOTE.to_string()
            },
            "the image block becomes the note"
        );
        assert_eq!(blocks.len(), replaced.fragments()[0].content().len());
        assert_eq!(
            replaced.fragments()[0].source(),
            &ContentSource::UserMessage,
            "attribution is preserved"
        );
        assert_eq!(
            replaced.fragments()[0].role(),
            Role::User,
            "role is preserved"
        );
        assert_eq!(
            replaced.fragments()[1],
            request.fragments()[1],
            "fragments without images are untouched"
        );
    }
}
