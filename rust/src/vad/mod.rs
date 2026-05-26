//! Voice Activity Detection (Silero VAD via ONNX/candle).
//!
//! Detects speech segments in audio stream to avoid transcribing silence.

pub mod detector;
