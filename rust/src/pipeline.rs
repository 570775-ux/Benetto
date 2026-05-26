//! Streaming pipeline orchestrator.
//!
//! Architecture: Audio thread → mpsc::channel → Transcriber thread → Result accumulator.
//! No locks in the hot path — channels + ring buffer ensure lock-free audio capture.

pub fn start_streaming(app: &std::sync::Arc<parking_lot::RwLock<crate::state::AppState>>) {
    // TODO: Spawn audio thread → mpsc channel → transcriber thread
}

pub fn stop_streaming(app: &std::sync::Arc<parking_lot::RwLock<crate::state::AppState>>) -> String {
    // TODO: Signal stop → drain channel → finalize transcription → return text
    String::new()
}
