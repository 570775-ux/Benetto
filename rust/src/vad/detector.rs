//! Silero VAD detector — ONNX inference via candle.

/// Detects whether an audio chunk contains speech.
pub fn is_speech(_samples: &[f32], _sample_rate: u32) -> bool {
    // TODO: Run Silero VAD ONNX model
    true // Placeholder — assume always speech for now
}
