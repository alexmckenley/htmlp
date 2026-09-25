use super::{ContentBlock, ContentSource, ModelRequest};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, num::NonZeroUsize};

/// Explicit heuristic policy. Image transport bytes are never treated as text.
/// This estimate excludes provider framing and is not a tokenizer count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ByteEstimator {
    pub bytes_per_token: NonZeroUsize,
    pub image_tokens: u64,
}
impl Default for ByteEstimator {
    fn default() -> Self {
        Self {
            bytes_per_token: NonZeroUsize::new(4).unwrap(),
            image_tokens: 8192,
        }
    }
}
impl ByteEstimator {
    /// Round up so nonempty text never disappears from accounting.
    pub fn text_tokens(&self, text: &str) -> u64 {
        self.byte_tokens(text.len())
    }
    fn byte_tokens(&self, bytes: usize) -> u64 {
        bytes.div_ceil(self.bytes_per_token.get()) as u64
    }
    /// Aggregate bytes by the full typed source before rounding each source
    /// once. The total is exactly the sum of these non-overlapping categories.
    pub fn estimate(&self, request: &ModelRequest) -> RequestEstimate {
        let mut sources: BTreeMap<ContentSource, (usize, u64)> = BTreeMap::new();
        for fragment in request.fragments() {
            let (bytes, images) = sources.entry(fragment.source().clone()).or_default();
            for block in fragment.content() {
                match block {
                    ContentBlock::Image { .. } => *images = images.saturating_add(1),
                    ContentBlock::Text { .. }
                    | ContentBlock::Reasoning { .. }
                    | ContentBlock::ToolUse { .. }
                    | ContentBlock::ToolResult { .. } => {
                        *bytes = bytes.saturating_add(block.byte_len());
                    }
                }
            }
        }
        if !request.tools().is_empty() {
            let (bytes, _) = sources.entry(ContentSource::ToolDefinitions).or_default();
            for tool in request.tools() {
                *bytes = bytes.saturating_add(tool.byte_len());
            }
        }
        let sources = sources
            .into_iter()
            .map(|(source, (text_bytes, images))| SourceEstimate {
                source,
                text_bytes,
                images,
                tokens: self
                    .byte_tokens(text_bytes)
                    .saturating_add(images.saturating_mul(self.image_tokens)),
            })
            .collect();
        RequestEstimate {
            method: *self,
            sources,
        }
    }
}

/// One source's heuristic allocation, not provider-reported category usage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SourceEstimate {
    pub source: ContentSource,
    pub text_bytes: usize,
    pub images: u64,
    pub tokens: u64,
}

/// Categorized estimate. Total derives from categories instead of being a
/// separately mutable field that could drift from them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RequestEstimate {
    method: ByteEstimator,
    sources: Vec<SourceEstimate>,
}
impl RequestEstimate {
    pub fn method(&self) -> ByteEstimator {
        self.method
    }
    pub fn sources(&self) -> &[SourceEstimate] {
        &self.sources
    }
    pub fn total_tokens(&self) -> u64 {
        self.sources
            .iter()
            .fold(0u64, |total, item| total.saturating_add(item.tokens))
    }
    pub fn tokens_for(&self, source: &ContentSource) -> u64 {
        self.sources
            .iter()
            .find(|s| &s.source == source)
            .map_or(0, |s| s.tokens)
    }
}

/// Provider-reported usage for one model call. Input/cache lanes are disjoint;
/// reasoning is a subset of output and must not be added to the output total.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ReportedUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
    pub reasoning_tokens: u64,
}
impl ReportedUsage {
    pub fn prompt_tokens(&self) -> u64 {
        self.input_tokens
            .saturating_add(self.cache_read_tokens)
            .saturating_add(self.cache_write_tokens)
    }
    /// Merge cumulative, partially populated snapshots of the SAME call.
    /// Do not use this to aggregate distinct requests.
    pub fn merge_snapshot(&mut self, snapshot: &Self) {
        self.input_tokens = self.input_tokens.max(snapshot.input_tokens);
        self.output_tokens = self.output_tokens.max(snapshot.output_tokens);
        self.cache_read_tokens = self.cache_read_tokens.max(snapshot.cache_read_tokens);
        self.cache_write_tokens = self.cache_write_tokens.max(snapshot.cache_write_tokens);
        self.reasoning_tokens = self.reasoning_tokens.max(snapshot.reasoning_tokens);
    }
    /// Sum distinct calls. Reasoning stays an explanatory subset of output.
    pub fn add_call(&mut self, call: &Self) {
        self.input_tokens = self.input_tokens.saturating_add(call.input_tokens);
        self.output_tokens = self.output_tokens.saturating_add(call.output_tokens);
        self.cache_read_tokens = self
            .cache_read_tokens
            .saturating_add(call.cache_read_tokens);
        self.cache_write_tokens = self
            .cache_write_tokens
            .saturating_add(call.cache_write_tokens);
        self.reasoning_tokens = self.reasoning_tokens.saturating_add(call.reasoning_tokens);
    }
}

/// Usage for one call. `estimate: None` means source attribution is unavailable
/// (for example, a backend assembled the prompt), not zero category usage.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct TokenUsage {
    pub estimate: Option<RequestEstimate>,
    pub reported: Option<ReportedUsage>,
    pub context_window_tokens: u64,
}
impl TokenUsage {
    /// Best available prompt fill: reported input including cache lanes,
    /// otherwise the categorized estimate. None means no measurement exists.
    pub fn prompt_tokens(&self) -> Option<u64> {
        self.reported
            .as_ref()
            .map(ReportedUsage::prompt_tokens)
            .or_else(|| self.estimate.as_ref().map(RequestEstimate::total_tokens))
    }
}
