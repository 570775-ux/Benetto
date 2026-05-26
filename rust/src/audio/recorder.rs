//! Android Oboe/AAudio wrapper.
//!
//! Captures audio at 16 kHz mono 16-bit PCM via Android's low-latency audio API.
//! Feeds a lock-free ring buffer consumed by the transcription thread.

// TODO: oboe-rs or raw AAudio bindings via jni
// For now: placeholder — actual implementation needs Android NDK
