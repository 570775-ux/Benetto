//! whisper-rs engine wrapper.

use crate::state::Error;

pub struct Engine;

impl Engine {
    pub fn load(model_path: &str) -> Result<(), Error> {
        // TODO: whisper_rs::WhisperContext::new(model_path)
        Ok(())
    }

    pub fn transcribe_chunk(&self, _audio: &[f32]) -> String {
        // TODO: whisper full → return partial text
        String::new()
    }
}
