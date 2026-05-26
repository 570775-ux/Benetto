//! Application state — shared across all threads via Arc<RwLock<AppState>>.

use crate::{pipeline, whisper, llm, audio};

/// Tracks whether models are loaded and pipeline is active.
#[derive(Default)]
pub struct AppState {
    initialized: bool,
    llm_loaded: bool,
    recording: bool,
    transcribing: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Whisper model not loaded")]
    WhisperNotLoaded,
    #[error("LLM model not loaded")]
    LLMNotLoaded,
    #[error("Audio device error: {0}")]
    Audio(String),
}

impl AppState {
    pub fn init(&mut self, model_path: &str, llm_path: &str) -> Result<(), Error> {
        whisper::Engine::load(model_path)?;
        llm::Engine::load(llm_path)?;
        self.initialized = true;
        self.llm_loaded = true;
        Ok(())
    }

    pub fn is_initialized(&self) -> bool { self.initialized }
    pub fn is_llm_loaded(&self) -> bool { self.llm_loaded }

    pub fn status(&self) -> i32 {
        if self.recording { return 1; }
        if self.transcribing { return 2; }
        0
    }

    pub fn mark_recording(&mut self) { self.recording = true; }
    pub fn mark_idle(&mut self) { self.recording = false; self.transcribing = false; }

    pub fn release(&mut self) {
        self.initialized = false;
        self.llm_loaded = false;
        self.recording = false;
        self.transcribing = false;
    }
}
