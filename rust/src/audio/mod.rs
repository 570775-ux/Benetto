//! Audio capture via Android Oboe/AAudio.
//!
//! Provides a ring buffer for continuous audio capture at 16 kHz, 16-bit mono.
//! The audio thread writes to the ring buffer, the transcriber thread reads from it.

pub mod recorder;
