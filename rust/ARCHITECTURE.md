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

### ADR-04: Ring buffer, not Vec<f32> for audio
**Decision:** Use `ringbuf` crate (lock-free SPSC ring buffer) for audio capture.
**Rationale:** Vec требует переаллокации при росте → фрагментация памяти + GC-паузы. Ring buffer pre-allocated, фиксированного размера, lock-free.
**Size:** 30 секунд × 16000 Hz × 2 байта (i16) × 1 канал = 960 KB. Умещается в L3-кэш.
**Alternatives considered:** `Vec<f32>` (растёт, фрагментирует), `Arc<Mutex<VecDeque>>` (блокировки в hot path).
**Decision:** Use `whisper-rs` crate (Rust bindings to whisper.cpp) instead of raw C++.
**Rationale:** Rust ownership model catches buffer errors. No `unsafe` in business logic.
**Alternatives considered:** Raw C++ via `cc` crate — rejected, defeats memory safety purpose.

## Data Flow: Streaming Transcription (v2 — bug-proof)

### The 7 Deadly Bugs of Streaming ASR

#### Bug 1: Word Splitting at Chunk Boundary
**Расколотое слово:** «сего» (конец чанка N) + «дня» (начало чанка N+1) → «сегодня»
**Fix:** Overlap 1-2 секунды между чанками. При склейке — ищем дублирующиеся токены в overlap-зоне и удаляем.
**Rust гарантия:** `&[f32]` immutable borrow — transcriber НЕ видоизменяет аудио-буфер пока recorder пишет. Два трека: `write_ptr` и `read_ptr` в ring buffer, никогда не пересекаются.

#### Bug 2: Duplicate Words from Overlap
**Дубликаты:** «сегодня сегодня был дождь»
**Fix:** Token-level dedup. Последние N токенов чанка N сравниваем с первыми N токенов чанка N+1. Удаляем дубликаты из начала чанка N+1 перед склейкой.
**Rust гарантия:** `std::collections::VecDeque<String>` для последних токенов. `pop_front()` при переполнении → O(1), нет утечки памяти.

#### Bug 3: Ring Buffer Overflow
**Потеря сэмплов:** аудио пишется быстрее чем whisper читает → сэмплы дропаются → дыры в транскрипции
**Fix:** `mpsc::sync_channel(2)` — канал с capacity=2. Если transcriber не успевает, `send()` блокирует recorder. Это backpressure: recorder ждёт, не дропает.
**Rust гарантия:** `sync_channel` на уровне типов — компилятор проверяет что sender не может отправить если receiver не готов.

#### Bug 4: VAD Cutoff
**Ложный negative:** пауза в 0.5 сек → VAD говорит «нет речи» → чанк пропущен → потеряно предложение
**Fix:** Graceful degradation. Если VAD не загружен или threshold не настроен → транскрибируем ВСЕ чанки. VAD — опциональная оптимизация, не gate.
**Rust гарантия:** `Option<VAD>` — если `None`, код даже не пытается вызвать `is_speech()`.

#### Bug 5: Whisper Context Contamination
**Контекстная грязь:** чанк N+1 «помнит» что было в чанке N → галлюцинации, повтор фраз
**Fix:** whisper-rs stateless mode. Каждый чанк → новый `whisper_full()` вызов, без сохранения состояния между чанками.
**Rust гарантия:** `whisper_full_params` с `WHISPER_AHEAD_NO_CONTEXT`. Компилятор гарантирует что состояние не протекает между вызовами.

#### Bug 6: JNI Deadlock
**Деадлок:** Java-поток вызывает `stopRecording()` → ждёт мьютекс → но Rust-поток держит мьютекс и ждёт Java callback → deadlock
**Fix:** JNI bridge только отправляет/читает из каналов. Никогда не держит блокировку дольше чем `send()`/`try_recv()`.
**Rust гарантия:** `Arc<RwLock<AppState>>` — read lock для status check, write lock только на init/release. Никогда не держим write lock в JNI-callable функциях.

#### Bug 7: Chunk Accumulation OOM
**Утечка памяти:** 30+ минут записи → все чанки в Vec → OOM
**Fix:** Stream results. После склейки чанка в output → удаляем аудио-чанк. Храним только:
- `output_text: String` (накопительный результат)
- `last_tokens: VecDeque<String>` (последние 20 токенов для dedup, ~2 KB)
- NO audio history
**Rust гарантия:** `VecDeque::pop_front()` при `len() > 20` — константная память независимо от длительности записи.

### Pipeline States (Finite State Machine)

```rust
enum PipelineState {
    Idle,                          // Нет записи
    Recording {                    // Идёт захват аудио
        recorder: RecorderHandle,  //   → пишет в ring buffer
        chunk_tx: SyncSender,      //   → отправляет чанки
    },
    Finalizing,                    // Остановка: ждём последний чанк
    Error(String),                 // Ошибка: причина
}

impl PipelineState {
    fn transition(&mut self, event: PipelineEvent) -> Result<PipelineState, Error> {
        match (self, event) {
            (Idle, StartRecording) => { spawn_recorder(); Ok(Recording {..}) }
            (Recording {..}, StopRecording) => { signal_stop(); Ok(Finalizing) }
            (Finalizing, ChunkComplete) => { drain_channel(); Ok(Idle) }
            (_, Error(e)) => Ok(Error(e)),
            _ => Err(Error::InvalidTransition),
        }
    }
}
```

### Гарантии (формально)

| Свойство | Как обеспечено | Уровень |
|----------|---------------|---------|
| Нет data races | `&[f32]` immutable в transcriber. Два указателя в ringbuf. | Compile-time |
| Нет deadlocks | Write lock < 1μs. Read lock для status. | Design |
| Нет OOM | `VecDeque::pop_front` при len>20. Чанки удаляются после склейки. | Runtime |
| Нет потери сэмплов | `sync_channel(2)` — backpressure на recorder | Runtime |
| Нет дубликатов | Token dedup в overlap-зоне | Algorithm |
| Нет расколотых слов | Overlap 1-2 сек + token matching | Algorithm |
| Graceful VAD fallback | `Option<VAD>` → всегда транскрибируем если None | Type system |

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

---

## SOS Module: Emergency Pipeline (ADR-05)

### User Flow (from original Benetto review)
```
1. 5 power-button taps → EmergencyService starts (foreground)
2. SMS to trusted contacts with GPS location → starts recording
3. Streaming transcription → keyword detection
4. Every 5 min: partial SMS (location + transcription snippet)
5. On stop: final SMS with full transcription
6. After reboot: SosBootReceiver restarts service
```

### Critical Bugs in Original → Rust Fixes

| Bug | Severity | Rust Fix |
|-----|----------|----------|
| EmergencyReceiver never unregistered → OOM | CRITICAL | `Drop` trait auto-cleans when service stops |
| `exported=false` → no reboot survival | CRITICAL | Manifest fix + `Option<BootState>` recovery check |
| `sendInitialSOS()` no retry → life-critical SMS lost | MAJOR | `retry(3, backoff=1s,4s,16s)` via `Result<Sent,Failed>` |
| Location can be 2h stale → user searches wrong place | MAJOR | `LocationAge` enum: Fresh(<30s), Recent(<5m), Stale(>5m) |
| Battery opt not tracked → app killed mid-SOS | MINOR | `BatteryState::is_exempted: bool` checked on start |

### SOS in Rust Pipeline

```
JNI: activateSos(contacts, userName, keywords)
       ↓
SosOrchestrator::start()
  ├── SmsNotifier::send_initial(contacts, location, userName)  [retry×3]
  ├── audio::Recorder::start() → ring_buffer → chunk_tx
  ├── LocationTracker::start_periodic(5min) → LocationAge enum
  ├── KeywordDetector::start(keywords + silence>30s)
  └── tokio::spawn(PeriodicSmsTask every 5min) → partial text + location → SMS
```

### New Types

```rust
pub struct SosConfig {
    pub contacts: Vec<TrustedContact>,
    pub user_name: String,
    pub interval: Duration,          // Partial SMS interval: 5 min
    pub silence_timeout: Duration,   // Auto-SOS on silence: 30 sec
    pub keywords: Vec<String>,       // ["помогите", "sos", "помощь"]
}

pub enum LocationAge {
    Fresh(Duration),    // < 30s — "📍"
    Recent(Duration),   // < 5min — "📍 ~5min ago"
    Stale(Duration),    // > 5min — "⚠️ STALE: 2h"
}

pub enum SosState {
    Idle,
    Active { start: Instant, last_sms: Instant, chunks: u32, keywords: Vec<String> },
    Error(SosError),
}

pub enum SosError {
    NoContacts, SmsFailed { contact: String, attempt: u32 },
    RecorderFailed, TranscriptionFailed,
}
```

### SOS Quality Gates

| Gate | Test |
|------|------|
| **SMS retry** | Fail ×3 with backoff → verify error logged, attempt counter |
| **Location stale** | GPS age > 5min → SMS includes "⚠️ STALE LOCATION" |
| **Memory leak** | Start/stop SOS 100× → RAM returns to baseline ± 5 MB |
| **Silence detect** | Feed 30s silence → auto-SOS trigger |
| **Keyword match** | Feed audio with "помогите" → flag in keywords list |
| **Cold start SMS** | SOS within 3s of boot → SMS sent with "(after reboot)" |
| **No contacts** | empty contacts → SosState::Error(NoContacts), no crash |

### ADR-05: SOS is a first-class pipeline mode, not an addon
**Decision:** SOS reuses the same streaming pipeline as normal transcription.
**Rationale:** Original Benetto had SOS as a separate code path → duplication, bugs only in SOS path. New architecture: SOS = PipelineMode::Emergency with additional periodic SMS task.
**Impact:** Bug fixes in streaming pipeline automatically fix SOS. No separate whisper context, no duplicate audio capture.

### SOS Activation: Two Entry Points

```
┌─────────────────────┐     ┌──────────────────────┐
│  In-App SOS Button   │     │  5 Power Button Taps │
│  (SosScreen.kt)      │     │  (SosBootReceiver)   │
│  User: "I need help" │     │  System-level panic  │
└─────────┬───────────┘     └──────────┬───────────┘
          │                            │
          └──────────┬─────────────────┘
                     ↓
          EmergencyService.start(sosConfig)
                     ↓
          SosOrchestrator::start(Rust)
                     ↓
          ┌──────────┴──────────┐
          │  streaming pipeline  │
          │  + periodic SMS      │
          └─────────────────────┘
```

**In-app activation:** User opens Benetto → taps SOS button → SosScreen with countdown (3s) → confirmation → EmergencyService starts.
**System activation:** 5 power button taps detected by `SosBootReceiver` → EmergencyService starts immediately (no confirmation).
**Rust side:** Same `SosOrchestrator::start()` regardless of entry point. `SosConfig.activation_source` distinguishes for logging.

### New JNI Export

```kotlin
// Added to BenettoNative.kt
external fun activateSos(
    contactsJson: String,     // JSON array of {name, phone}
    userName: String,
    activationSource: Int     // 0=in-app, 1=power-button
): Int                        // Returns SosState ordinal
```

### SOS Activation: Two Entry Points

```
In-App SOS Button                 5 Power Button Taps
(SosScreen.kt, countdown 3s)      (SosBootReceiver, immediate)
         │                                │
         └────────────┬───────────────────┘
                      ↓
           EmergencyService.start(config)
                      ↓
           SosOrchestrator::start()  ← SAME Rust pipeline
                      ↓
           ┌─────────┴─────────┐
           │ streaming pipeline │
           │ + periodic SMS     │
           └───────────────────┘
```

**In-app:** User opens Benetto → SOS tab → countdown 3s → confirm → start.
**Power button:** 5 taps → no confirmation → immediate start.
**Rust side:** Same `SosOrchestrator::start()` for both. `activation_source: 0|1` in config for logging.

### New JNI Export for SOS

```kotlin
// Added to BenettoNative.kt
external fun activateSos(contactsJson: String, userName: String, source: Int): Int
// Returns: 0=Idle, 1=Active, -1=Error(NoContacts), -2=Error(RecorderFailed)
```

---

## Gap Analysis: Original Benetto vs Rust Architecture

### Covered ✅
| Original | Rust Equivalent | Improvement |
|----------|----------------|-------------|
| WhisperPipeline.kt (6/10) | whisper/engine.rs | Compile-time safety, no JNI overhead |
| RealWhisperPipeline.kt (8.5/10) | Merged into whisper/engine.rs | One implementation, not two |
| VadPipeline.kt (6.5/10) | vad/detector.rs | Same VAD model, Rust wrapper |
| QwenPipeline.kt (8.5/10) | llm/engine.rs | Same model, native candle inference |
| FullPipelineManager.kt (8.5/10) | pipeline.rs | Streaming + FSM, no GC pauses |
| 11 SOS files | 4 SOS modules | Bug fixes embedded in design |
| whisper_jni.cpp + llama_jni.cpp | lib.rs + jni_bridge.rs | No double-free, no memory leaks |

### Not Covered (Kotlin/Compose stays) ⚠️
| Original | Status |
|----------|--------|
| 16 UI files (HomeScreen, RecordScreen, etc.) | Kotlin/Compose — stays as-is |
| AppDatabase.kt + RecordingRepository.kt | Room (SQLite) — stays on Kotlin |
| Navigation.kt | Jetpack Navigation — stays |

### Not Yet Covered ❌
| Feature | Original Score | Priority | Notes |
|---------|---------------|----------|-------|
| **Speaker Diarization** | 6/10 (buggy) | P2 | "Who spoke when" — needs pyannote or similar ONNX model. Add after whisper+llm work. |
| **Model Download Manager** | 8.5/10 (solid) | P1 | Download gguf/ggml from HuggingFace. Needed before APK release. Kotlin-side (DownloadManager API), but needs Rust callback for progress. |
| **Recording Repository** | 9.5/10 (solid) | P1 | Room DB for recordings history. Kotlin-side, but Rust needs to provide transcription text for storage. |

### ADR-06: Speaker Diarization — defer to v1.1
**Decision:** Diarization NOT in v1.0 Rust core. Original implementation was buggy (6/10).
**Rationale:** Adds ~200 MB (pyannote model), increases latency 2-3×. v1.0 focus: reliable streaming transcription + SOS.
**v1.1 plan:** ONNX-based speaker embedding model via candle, or pyannote-rs if available.

### ADR-07: Model Download Manager — Kotlin-side with Rust progress callback
**Decision:** Download logic in Kotlin (Android DownloadManager API), download progress + verification callback to Rust.
**Rationale:** Android DownloadManager handles WiFi-only, retry, notification. Rust receives model path and verifies checksum (SHA256) before loading.
**JNI interface:**
```kotlin
external fun verifyModel(path: String): Boolean  // SHA256 check
external fun getModelVersion(path: String): String  // Returns "small" / "qwen-0.5b"
```
