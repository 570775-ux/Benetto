//! Benetto-core — native Rust audio processing for Android.
//!
//! Architecture: JNI bridge → streaming pipeline → whisper-rs + candle LLM.
//! Protocol: Mauri2 pattern (channels, no unsafe, compile-time safety).

mod jni_bridge;
mod pipeline;
mod audio;
mod whisper;
mod llm;
mod vad;
mod state;

use std::sync::Arc;
use parking_lot::RwLock;
use state::AppState;

/// Global application state.
static APP: once_cell::sync::Lazy<Arc<RwLock<AppState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(AppState::default())));

// ── JNI exports ──────────────────────────────────────────────

use jni::JNIEnv;
use jni::objects::{JClass, JString};
use jni::sys::{jboolean, jint, jstring};

/// Initialize Benetto: load models, set up audio.
#[no_mangle]
pub extern "system" fn Java_com_whispercppdemo_BenettoNative_init(
    mut env: JNIEnv,
    _class: JClass,
    model_path: JString,
    llm_path: JString,
) -> jboolean {
    let model: String = env.get_string(&model_path)
        .expect("model_path must be valid UTF-8")
        .into();
    let llm: String = env.get_string(&llm_path)
        .expect("llm_path must be valid UTF-8")
        .into();

    let mut state = APP.write();
    match state.init(&model, &llm) {
        Ok(()) => jni::sys::JNI_TRUE,
        Err(e) => {
            log::error!("Init failed: {}", e);
            jni::sys::JNI_FALSE
        }
    }
}

/// Start recording and streaming transcription.
#[no_mangle]
pub extern "system" fn Java_com_whispercppdemo_BenettoNative_startRecording(
    _env: JNIEnv,
    _class: JClass,
) -> jboolean {
    let state = APP.read();
    if state.is_initialized() {
        pipeline::start_streaming(&APP);
        jni::sys::JNI_TRUE
    } else {
        jni::sys::JNI_FALSE
    }
}

/// Stop recording, finish transcription, return full text.
#[no_mangle]
pub extern "system" fn Java_com_whispercppdemo_BenettoNative_stopRecording<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
) -> jstring {
    let text = pipeline::stop_streaming(&APP);
    env.new_string(text)
        .expect("Failed to create Java string")
        .into_raw()
}

/// Get status: 0=idle, 1=recording, 2=transcribing.
#[no_mangle]
pub extern "system" fn Java_com_whispercppdemo_BenettoNative_getStatus(
    _env: JNIEnv,
    _class: JClass,
) -> jint {
    let state = APP.read();
    state.status() as jint
}

/// LLM summarization.
#[no_mangle]
pub extern "system" fn Java_com_whispercppdemo_BenettoNative_summarize<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    text: JString<'local>,
) -> jstring {
    let input: String = env.get_string(&text)
        .unwrap_or_default()
        .into();
    let summary = llm::summarize(&APP, &input);
    env.new_string(summary)
        .expect("Failed to create Java string")
        .into_raw()
}

/// Check if LLM is loaded.
#[no_mangle]
pub extern "system" fn Java_com_whispercppdemo_BenettoNative_isLLMLoaded(
    _env: JNIEnv,
    _class: JClass,
) -> jboolean {
    let state = APP.read();
    if state.is_llm_loaded() { jni::sys::JNI_TRUE } else { jni::sys::JNI_FALSE }
}

/// Release all resources.
#[no_mangle]
pub extern "system" fn Java_com_whispercppdemo_BenettoNative_release(
    _env: JNIEnv,
    _class: JClass,
) {
    let mut state = APP.write();
    state.release();
}
