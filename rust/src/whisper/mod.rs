//! Whisper inference engine.
//!
//! Wraps whisper-rs for on-device speech-to-text.
//! Supports streaming mode (partial results) via chunk-based processing.

pub mod engine;
