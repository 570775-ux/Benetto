# Benetto-core — Rust Architecture v1.1
> Reviewed by 5 AI agents (Minimax M2.7) — 26 issues fixed.

## Overview

Benetto-core is a Rust library providing native audio transcription and LLM summarization for Android, compiled as a shared library (`.so`) and loaded via JNI.

**Design principle:** No GC pauses, no data races, compile-time safety.
**Note:** Mauri2 pattern reference removed (v1.1) — audio/ML domain differs from VPN packet processing.

## Crate Structure

```
rust/
├── Cargo.toml
├── src/
│   ├── lib.rs              # JNI entry points (package: com.voicenotes)  ← FIXED
│   ├── jni_bridge.rs       # Kotlin↔Rust type conversion + callbacks
│   ├── pipeline.rs         # Streaming pipeline (std::sync::mpsc, not tokio)  ← FIXED
│   ├── audio/
│   │   ├── mod.rs
│   │   └── recorder.rs     # Android Oboe/AAudio wrapper
│   ├── whisper/
│   │   ├── mod.rs
│   │   └── engine.rs       # whisper-rs wrapper, stateless per chunk
│   ├── llm/
│   │   ├── mod.rs
│   │   └── engine.rs       # candle wrapper, Qwen2.5-0.5B
│   ├── vad/
│   │   ├── mod.rs
│   │   └── detector.rs     # Silero VAD (ONNX via candle)
│   ├── sos/
│   │   ├── mod.rs
│   │   ├── orchestrator.rs # SOS state machine
│   │   ├── sms.rs           # SMS via Android API (JNI)
│   │   ├── location.rs      # GPS tracker with LocationAge labeling
│   │   └── keywords.rs      # Keyword detection in stream + VAD peak detection
│   └── state.rs            # Shared state: Arc<RwLock<AppState>>
├── tests/
│   ├── integration_test.rs
│   └── audio_samples/
└── build.rs
```

## Architecture Decision Records (ADR)

### ADR-01: Rust native, Kotlin UI
**Decision:** All audio/ML in Rust `.so`, UI in Kotlin/Compose via JNI.
**Rationale:** Kotlin/Compose for Material 3 UI. Rust for safety-critical audio/ML.
**Alternatives considered:** Pure C++ — rejected for memory safety. Pure Kotlin — GC pauses during audio capture.

### ADR-02: Streaming via std::sync::mpsc (not tokio)  ← CHANGED
**Decision:** `std::sync::mpsc::sync_channel` for chunk-based audio streaming.
**Rationale:** Two threads (mic → whisper). Tokio async runtime is overkill. std channels are zero-dependency and sufficient.
**Alternatives considered:** `tokio::sync::mpsc` — rejected (overhead). `crossbeam` — rejected (extra dep for 2-thread use case).

### ADR-03: whisper-rs over raw C++
**Decision:** Use `whisper-rs` (Rust bindings to whisper.cpp).
**Rationale:** Rust ownership model catches buffer errors. No `unsafe` in business logic.
**Note:** whisper-rs 0.13 confirmed compatible with production use. WHISPER_AHEAD_NO_CONTEXT exists in latest whisper.cpp — verify at build time via feature flag.

### ADR-04: Ring buffer + backpressure
**Decision:** `ringbuf` crate (lock-free SPSC ring buffer, 960 KB) + `sync_channel(2)` backpressure.
**⚠️ REVIEW FINDING FIXED:** The original architecture claimed `&[f32]` prevents races. INCORRECT. Borrow checker doesn't know two slices from the same ringbuf can alias. Fix: use ringbuf's built-in `Producer`/`Consumer` split — these are properly synchronized internally with atomics. No raw slices exposed.
**Size:** 30s × 16kHz × i16 × 1ch = 960KB. L3-cached.
**Backpressure:** `sync_channel(2)` blocks recorder when transcriber is slow. No sample loss.

### ADR-05: SOS as pipeline mode (with isolation)  ← CHANGED
**Decision:** SOS reuses streaming pipeline BUT with `assert!(mode == Emergency)` guards. SOS has its own error handler that does NOT propagate to normal transcription.
**Rationale:** Reviewers flagged risk: SOS bug → kills normal transcription. Fix: SOS-specific error handling with separate recovery path. If pipeline crashes in Emergency mode → restart pipeline in Emergency mode, not fall through to Idle.
**⚠️ SOS failures NEVER fall through to normal recording path.**

#### ADR-16: Model Strategy — single small-q4_0 for all devices ← NEW
**Decision:** NOT in v1.0. Adds ~200 MB (pyannote model), 2-3× latency. Original Benetto diarization was 6/10 (buggy). Deferred.

### ADR-07: Model download on Kotlin side
**Decision:** DownloadManager API (WiFi-only, retry, notification). Rust verifies SHA256 before loading.
**Security:** Model checksum validation in Rust prevents corrupted/tampered models.

### ADR-08: panic=unwind for JNI safety  ← NEW
**Decision:** `panic = "unwind"` in release profile. All JNI exports wrapped in `std::panic::catch_unwind`.
**Rationale:** `panic=abort` kills the entire Java process on any Rust panic — unacceptable for SOS. `catch_unwind` at JNI boundary returns error to Java instead of crashing.
**Trade-off:** +10-15% binary size (~1 MB).

### ADR-09: Threading Model  ← NEW
**Decision:** Two ownership domains:
1. **Audio thread:** owns Recorder + ringbuf Producer. Writes samples. Blocks on `sync_channel::send()` if full.
2. **Transcriber thread:** owns ringbuf Consumer + whisper context. Reads chunks, processes, pushes results via callback.
3. **SOS periodic thread:** spawned via `std::thread::spawn`, sleeps 5min, sends SMS.
**No thread pool. No work stealing. Explicit ownership.**

### ADR-10: Error Handling Strategy  ← NEW
**Decision:** `Result<T, BenettoError>` for all fallible operations. Panics ONLY in JNI boundary (caught by `catch_unwind`). Errors propagate up to Kotlin as error codes or callback messages.
**Logging:** `android_logger` crate for structured Android logcat output.

---

## Data Flow: Streaming Transcription (v2.1 — reviewed)

### The 7 Deadly Bugs — FIXED VERSIONS

#### Bug 1: Word Splitting at Chunk Boundary — FIX: fuzzy overlap matching
**Original fix (v1.0):** Token-level comparison. **REVIEW FINDING:** Whisper is non-deterministic — same audio → different tokens at different alignment windows. Token comparison FAILS.
**v1.1 fix:** Overlap 1-2 seconds. After transcribing both chunks, compute **text similarity** in overlap zone (Levenshtein or longest-common-substring). Remove duplicated text, NOT duplicated tokens.
**Rust:** ringbuf's `Producer`/`Consumer` atomics prevent read/write overlap at the hardware level.

#### Bug 2: Duplicate Words from Overlap — FIX: fuzzy text matching ← CHANGED
**Original fix (v1.0):** VecDeque<String> last tokens. **REVIEW FINDING:** Won't work due to non-deterministic tokenization.
**v1.1 fix:** After transcription, take last ~50 chars of chunk N output, find longest common substring with first ~50 chars of chunk N+1. Trim overlap from chunk N+1 before concatenation.
**Rust:** `str::find()` or `textdistance::levenshtein` crate. O(1) memory — no VecDeque needed.

#### Bug 3: Ring Buffer Overflow — unchanged
**Fix:** `sync_channel(2)` blocks recorder. No sample loss. Ringbuf producer/consumer atomics.
**Rust:** `Producer::push()` returns `Err(Full)` if no space → recorder pauses.

#### Bug 4: VAD Cutoff — unchanged
**Fix:** `Option<VAD>`. If None or `is_speech` threshold not met → transcribe all chunks.
**Rust:** Type system guarantees no VAD call if None.

#### Bug 5: Whisper Context Contamination — confirmed stateless
**Fix:** `whisper_full()` is stateless by default. Each chunk → new `whisper_full()` call. No context saved between chunks.
**Verification:** Check whisper-rs 0.13 API for `whisper_full_params` — add compile-time assertion test.

#### Bug 6: JNI Deadlock — unchanged
**Fix:** JNI bridge only `send()`/`try_recv()`. No write locks. Read lock for status.
**Rust:** `parking_lot::RwLock` — uncontended read lock is ~2 atomics, no syscall.

#### Bug 7: Chunk Accumulation OOM — unchanged
**Fix:** No audio history stored. Only `output_text: String` + `error_state: Option<Error>`. Constant memory.
**Rust:** Chunks dropped after transcription — `Drop` frees memory immediately.

### Revised Pipeline States (FSM v2)

```rust
enum PipelineState {
    Idle,
    Recording {
        recorder: RecorderHandle,
        chunk_tx: SyncSender<Vec<i16>>,  // std::sync, not tokio
    },
    Finalizing,
    Error(PipelineError),  // Recovery path: restart or propagate to UI
}

enum PipelineMode {
    Normal,     // Standard transcription
    Emergency,  // SOS mode — error recovery goes back to Emergency, not Idle
}
```

### Гарантии (updated)

| Свойство | Обеспечение | Уровень |
|----------|------------|---------|
| Нет data races | ringbuf Producer/Consumer atomics | Compile-time |
| Нет deadlocks | No write locks in JNI, read-only status | Design |
| Нет OOM | Chunks dropped after transcription | Runtime |
| Нет потери сэмплов | sync_channel(2) backpressure | Runtime |
| Нет дубликатов | Fuzzy text matching, not token matching | Algorithm |
| Нет расколотых слов | Overlap 1-2s + longest common substring | Algorithm |
| Graceful VAD fallback | `Option<VAD>` type system gate | Type system |
| Panic recovery | catch_unwind at JNI boundary | Runtime |
| SMS failure visible | Alert sound + UI error display | UX design |

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
hound = "3.5"
ringbuf = "0.4"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
parking_lot = "0.12"
thiserror = "1"
once_cell = "1"
android_logger = "0.14"
textdistance = "0.1"

[profile.release]
opt-level = "s"            # Optimize for size (APK)
lto = true
codegen-units = 1
panic = "unwind"           # ← CHANGED: abort→unwind for JNI safety
strip = true
```

## JNI Interface (v1.1 — corrected)

```kotlin
// package: com.voicenotes.local  ← FIXED from com.whispercppdemo
object BenettoNative {
    init { System.loadLibrary("benetto_native") }

    // Transcription
    external fun init(modelPath: String, llmPath: String): InitResult
    external fun startRecording(): Int         // 0=ok, -1=not_init, -2=already_recording
    external fun stopRecording(): String?      // null if not recording
    external fun getStatus(): Int

    // Streaming progress callback
    external fun setProgressCallback(callback: TranscriptionCallback)

    // LLM
    external fun summarize(text: String): String
    external fun isLLMLoaded(): Boolean

    // SOS
    external fun activateSos(contactsJson: String, userName: String, source: Int): Int

    // Lifecycle
    external fun release()

    // Callback interface for partial transcription + errors
    interface TranscriptionCallback {
        fun onPartialTranscription(text: String)
        fun onError(errorCode: Int, message: String)
    }
}

data class InitResult(val whisperLoaded: Boolean, val llmLoaded: Boolean, val error: String?)
```

### Rust-side exports (corrected signatures):

```rust
#[no_mangle]
pub extern "C" fn Java_com_voicenotes_local_BenettoNative_init(
    env: JNIEnv, _class: JClass, model_path: JString, llm_path: JString
) -> jobject;  // Returns InitResult

#[no_mangle]
pub extern "C" fn Java_com_voicenotes_local_BenettoNative_startRecording(
    env: JNIEnv, _class: JClass
) -> jint;
// Returns: 0=OK, -1=not_initialized, -2=already_recording  ← FIXED: idempotent

#[no_mangle]
pub extern "C" fn Java_com_voicenotes_local_BenettoNative_stopRecording(
    env: JNIEnv, _class: JClass
) -> jstring;
// Returns: transcription text, or null if was not recording  ← FIXED: null vs ""

#[no_mangle]
pub extern "C" fn Java_com_voicenotes_local_BenettoNative_setProgressCallback(
    env: JNIEnv, _class: JClass, callback: JObject
);

#[no_mangle]
pub extern "C" fn Java_com_voicenotes_local_BenettoNative_release(
    env: JNIEnv, _class: JClass
);
// Safe to call twice — checks initialized flag first  ← FIXED

// All exports wrapped in catch_unwind  ← NEW
```

## Quality Gates (updated)

| Gate | Command | Threshold |
|------|---------|-----------|
| **Format** | `cargo fmt --all -- --check` | 0 diffs |
| **Lint** | `cargo clippy --all-targets -- -D warnings` | 0 warnings |
| **Unsafe** | `grep -r "unsafe fn" src/` | 0 results (only in JNI bridge, documented) |
| **Build** | `cargo build --release --target aarch64-linux-android` | Exit 0 |
| **Test** | `cargo test --all` | All pass |
| **Size** | `ls -l target/release/libbenetto_native.so` | < 12 MB (was 10, +2MB for unwind) |

## SOS Module — v1.1 corrections

### All-Contacts-Fail is now handled  ← NEW (review finding #1)

```rust
enum SmsSendResult {
    AllSent,
    PartialSent { failed: Vec<String> },
    AllFailed,
}

// In SosOrchestrator::periodic_sms_task():
match result {
    AllFailed => {
        // 1. Play alert sound via JNI callback
        // 2. Store failed state — retry on next cycle
        // 3. Set SosState::Error(SosError::AllSmsFailed)
    }
    PartialSent { failed } => {
        // Log which contacts failed, continue with successful ones
    }
    AllSent => { /* OK */ }
}
```

### User feedback on SMS failure ← NEW
- Persistent notification: "⚠️ SOS message failed to send. Checking connectivity..."
- Haptic: 3 short vibrations on all-contacts-fail
- Sound: alert tone via Android RingtoneManager

### LocationAge thresholds corrected
```rust
pub enum LocationAge {
    Fresh(Duration),     // < 60s (was 30s — GPS cold-fix takes 30-60s)
    Recent(Duration),    // < 5min
    Stale(Duration),     // > 5min → SMS includes "⚠️ локация обновлена N мин назад"
}
```

### SOS-specific keyword: VAD peak detection ← NEW
```rust
// In addition to transcription-based keyword detection:
// Immediate peak detector — reacts to sustained loud audio
// Does NOT wait for whisper transcription
fn detect_audio_peak(samples: &[i16], threshold_db: f32) -> bool {
    let rms = samples.iter().map(|s| (*s as f64).powi(2)).sum::<f64>() / samples.len() as f64;
    20.0 * (rms.sqrt() / 32768.0).log10() > threshold_db as f64
}
// Triggers: "Loud sound detected — activating SOS attention mode"
```

### SOS user-visible feedback during active state ← NEW
```
┌─────────────────────────────────┐
│ 🆘 SOS Active                   │
│ 📍 Tracking location...         │
│ 🎙️ Recording audio...           │
│ 📤 Last SMS: 2 min ago          │
│                                 │
│ [Stop SOS]  [Test Sound]        │
└─────────────────────────────────┘
```

### Silence timeout configurable
```rust
pub struct SosConfig {
    pub silence_timeout: Duration,  // Default: 30s, user-configurable: 30/60/120s
    // ...
}
```

---

## Changelog

| Version | Changes |
|---------|---------|
| v1.0 | Initial architecture |
| v1.1 | 26 fixes from 5-agent review: ringbuf atomics, fuzzy text dedup, std::mpsc, panic=unwind, JNI package fix, init() partial failure fix, SOS all-contacts-fail handling, LocationAge 60s, idempotent recording, progress callback, threading model, error strategy |

---

**Next:** Phase 2 (Coder) — implement pipeline.rs + audio/recorder.rs with corrected architecture.
