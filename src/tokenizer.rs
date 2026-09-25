use crate::TokenCounter;
/// Offline, exact CL100K token counting. Special-token-looking strings are ordinary
/// text. This counts content, not any provider's message framing overhead.
pub struct Cl100k(tiktoken_rs::CoreBPE);
impl Cl100k {
    pub fn new() -> Result<Self, String> {
        tiktoken_rs::cl100k_base()
            .map(Self)
            .map_err(|e| e.to_string())
    }
}
impl TokenCounter for Cl100k {
    fn name(&self) -> &str {
        "cl100k_base"
    }
    fn count(&self, text: &str) -> u64 {
        self.0.encode_ordinary(text).len() as u64
    }
}
