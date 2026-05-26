//! candle-based LLM engine for Qwen2.5-0.5B.

use crate::state::Error;

pub struct Engine;

impl Engine {
    pub fn load(_model_path: &str) -> Result<(), Error> {
        // TODO: candle::Device::Cpu, load GGUF
        Ok(())
    }
}

/// Generate summary from transcribed text.
pub fn summarize(
    app: &std::sync::Arc<parking_lot::RwLock<crate::state::AppState>>,
    text: &str,
) -> String {
    if !app.read().is_llm_loaded() {
        return "LLM not available".to_string();
    }
    // TODO: Candle generation with Qwen chat template
    format!("[Summary of {} chars]", text.len())
}
