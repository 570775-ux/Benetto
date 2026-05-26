//! LLM inference engine (Qwen2.5-0.5B via candle).
//!
//! Provides on-device text summarization of transcriptions.

pub mod engine;
pub use engine::*;
