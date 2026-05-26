# Benetto-core — Rust Architecture v1.0

## Overview

Benetto-core is a Rust library providing native audio transcription and LLM summarization for Android, compiled as a shared library (`.so`) and loaded via JNI.

**Design principle:** Mauri2 architecture adapted for audio/ML. No GC pauses, no data races, compile-time safety.

## Crate Structure

```
rust/
├── Cargo.toml
├── src/
│   ├── lib.rs              # JNI entry points
│   ├── jni_bridge.rs       # Kotlin↔Rust type conversion
│   ├── pipeline.rs         # Streaming pipeline orchestrator
│   ├── audio/
│   │   ├── mod.rs
│   │   └── recorder.rs     # Android Oboe/AAudio wrapper
│   ├── whisper/
│   │   ├── mod.rs
│   │   └── engine.rs       # whisper-rs wrapper, streaming mode
│   ├── llm/
│   │   ├── mod.rs
│   │   └── engine.rs       # candle wrapper, Qwen2.5-0.5B
│   ├── vad/
│   │   ├── mod.rs
│   │   └── detector.rs     # Silero VAD (ONNX via candle)
│   └── state.rs            # Shared state: Arc<RwLock<AppState>>
├── tests/
│   ├── integration_test.rs
│   └── audio_samples/      # Test WAV files
└── build.rs                # Cross-compilation config
```

## Architecture Decision Records (ADR)

### ADR-01: Rust native, Kotlin UI
**Decision:** All audio/ML in Rust `.so`, UI in Kotlin/Compose via JNI.
**Rationale:** Mauri2 pattern works. Kotlin/Compose for Material 3 UI. Rust for everything safety-critical.
**Alternatives considered:** Pure C++ (whisper.cpp native) — rejected due to memory safety risks in streaming pipeline.

### ADR-02: Streaming pipeline via channels
**Decision:** `tokio::sync::mpsc` for chunk-based audio streaming.
**Rationale:** Rust channels prevent data races at compile time. No manual mutex locking.
**Alternatives considered:** `crossbeam` channels — rejected, tokio already used in Mauri2.

### ADR-03: whisper-rs over raw whisper.cpp
**Decision:** Use `whisper-rs` crate (Rust bindings to whisper.cpp) instead of raw C++.
**Rationale:** Rust ownership model catches buffer errors. No `unsafe` in business logic.
**Alternatives considered:** Raw C++ via `cc` crate — rejected, defeats memory safety purpose.

## Data Flow: Streaming Transcription

```
┌──────────────┐    mpsc::channel     ┌──────────────────┐
│ Audio Thread  │────────────────────▶│ Transcriber Thread │
│ (oboe)        │   Vec<f32> chunks    │ (whisper-rs)      │
│              │                      │                   │
│ RingBuffer   │                      │ process_chunk()    │
│ 30s, 16kHz  │                      │ overlap 1s         │
│              │                      │ dedup tokens       │
└──────────────┘                      └────────┬──────────┘
                                                │
                                      ┌────────▼──────────┐
                                      │ Result Thread      │
                                      │                   │
                                      │ Accumulate text    │
                                      │ JNI callback → UI  │
                                      └───────────────────┘
```

## Cargo.toml

```toml
[package]
name = "benetto-core"
version = "0.1.0"
edition = "2024"

[lib]
name = "benetto_native"
crate-type = ["cdylib"]

[dependencies]
jni = "0.21"
whisper-rs = "0.13"
candle-core = "0.7"
tokenizers = "0.21"
hound = "3.5"              # WAV encode/decode
tokio = { version = "1", features = ["rt", "sync"] }
ringbuf = "0.4"            # Lock-free ring buffer
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[profile.release]
opt-level = "s"            # Optimize for size (APK)
lto = true
codegen-units = 1
panic = "abort"            # No panic unwinding in .so
```

## JNI Interface (Kotlin side expects):

```kotlin
object BenettoNative {
    init { System.loadLibrary("benetto_native") }
    
    // Transcription
    external fun init(modelPath: String, llmPath: String): Boolean
    external fun startRecording(): Boolean
    external fun stopRecording(): String     // Returns transcription
    external fun getStatus(): Int            // 0=idle, 1=recording, 2=transcribing
    
    // LLM
    external fun summarize(text: String): String
    external fun isLLMLoaded(): Boolean
    
    // Lifecycle
    external fun release()
}
```

## API Contract

### Rust side exports (for JNI):
```rust
#[no_mangle]
pub extern "C" fn Java_com_whispercppdemo_BenettoNative_init(
    env: JNIEnv, _class: JClass, model_path: JString, llm_path: JString
) -> jboolean;

#[no_mangle]
pub extern "C" fn Java_com_whispercppdemo_BenettoNative_startRecording(
    env: JNIEnv, _class: JClass
) -> jboolean;

#[no_mangle]
pub extern "C" fn Java_com_whispercppdemo_BenettoNative_stopRecording(
    env: JNIEnv, _class: JClass
) -> jstring;

#[no_mangle]
pub extern "C" fn Java_com_whispercppdemo_BenettoNative_summarize(
    env: JNIEnv, _class: JClass, text: JString
) -> jstring;

#[no_mangle]
pub extern "C" fn Java_com_whispercppdemo_BenettoNative_release(
    env: JNIEnv, _class: JClass
);
```

## Quality Gates (coder-fleet)

| Gate | Command | Threshold |
|------|---------|-----------|
| **Format** | `cargo fmt --all -- --check` | 0 diffs |
| **Lint** | `cargo clippy --all-targets -- -D warnings` | 0 warnings |
| **Unsafe** | `grep -r "unsafe" src/` | 0 results (except JNI) |
| **Build** | `cargo build --release --target aarch64-linux-android` | Exit 0 |
| **Test** | `cargo test --all` | All pass |
| **Size** | `ls -l target/release/libbenetto_native.so` | < 10 MB |

## Next Steps

1. Phase 1 (ARCHITECT): ✅ This document
2. Phase 2 (CODER): Implement lib.rs + jni_bridge.rs + Cargo.toml
3. Phase 3 (CODER): Implement audio/recorder.rs (Android Oboe)
4. Phase 4 (CODER): Implement whisper/engine.rs
5. Phase 5 (CODER): Implement pipeline.rs (streaming)
6. Phase 6 (CODER): Implement llm/engine.rs
7. Phase 7 (REVIEWER): Full review cycle
8. Phase 8 (TESTER): Cross-compile + integration tests
