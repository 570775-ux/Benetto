# Техническое задание: Восстановление Benetto v2.0

**Статус:** Ожидает выполнения
**Создано:** 2026-05-26 23:20 GMT+3
**Контекст:** Исходный Kotlin-код утерян (rm -rf .git). Сохранились 200+ код-ревью файлов. Новая Rust-архитектура спроектирована и отревьюена 8 агентами.

---

## 🎯 Цель

Восстановить Benetto как работающее приложение, объединив:
1. **Логику из старого кода** (извлечённую из код-ревью)
2. **Новую архитектуру** (Rust, streaming, quantized models, SOS)
3. **Все 55+ правок** от 8 AI-агентов

## 📦 Исходные материалы

### Сохранились на диске:
- **200+ код-ревью файлов** (~2 MB, `review/`, `docs/reviews/`, `REVIEW_CYCLE_*.md`, `cycle-*.md`)
  - Описывают каждый файл: методы, баги, фиксы, архитектуру
  - 42 цикла ревью, ~270 найденных багов
  - Детальные фрагменты кода в markdown-блоках
- **ARCHITECTURE.md** v1.1 (Rust, 431 строк, 10 ADR)
- **lib.rs, state.rs, Cargo.toml, pipeline.rs** — стабы на Rust
- **Kotlin-UI** — whisper.cpp Android demo (рабочий, с Whisper Small + Qwen)

### Утеряно:
- Исходный Kotlin-код (`app/src/main/java/com/voicenotes/local/*.kt`)
- Git-история

## 🏗️ Что нужно восстановить

### Phase A: Извлечение старой логики из ревью

Для каждого файла из старого Benetto:
1. Прочитать ВСЕ ревью-файлы где он упоминается
2. Извлечь: сигнатуры методов, поля классов, алгоритмы, баги и фиксы
3. Собрать в спецификацию: что делал файл, какие методы, какие баги были исправлены

**Ключевые файлы для восстановления:**

| Домен | Файлы | Приоритет |
|-------|-------|-----------|
| **SOS** | SosOrchestrator, EmergencyService, SmsNotifier, LocationTracker, SosStateStore | P0 |
| **ML** | WhisperPipeline, RealWhisperPipeline, VadPipeline, QwenPipeline, FullPipelineManager | P0 |
| **DB** | AppDatabase, RecordingDao, EmergencyDao, RecordingEntity | P1 |
| **UI** | HomeScreen, RecordScreen, ResultScreen, SosScreen | P1 |
| **Native** | whisper_jni.cpp, llama_jni.cpp | P1 |

### Phase B: Адаптация под новую архитектуру

Что из старого кода НЕ переносить (новое лучше):
- ❌ Kotlin WhisperPipeline → заменено на Rust `whisper/engine.rs`
- ❌ Kotlin QwenPipeline → заменено на Rust `llm/engine.rs`
- ❌ Kotlin VadPipeline → заменено на Rust `vad/detector.rs`
- ❌ Булевы флаги состояния → заменено на Rust `PipelineState` enum + FSM
- ❌ JNI double-free (llama_jni.cpp) → исключено Rust ownership
- ❌ EmergencyReceiver leak → исключено `Drop` trait

Что из старого ПЕРЕНЕСТИ (бизнес-логика):
- ✅ SMS-формат: начальное, частичное, финальное сообщение
- ✅ SOS FSM: IDLE → RECORDING → TRANSCRIBING → FINALIZING
- ✅ Контакты: валидация телефона, лимиты, normalisation
- ✅ Location: fallback-цепочка GPS → lastKnown → graceful fail
- ✅ Permission model: audio, SMS, location с graceful degradation
- ✅ DB schema: таблицы, миграции, Room entities

### Phase C: Что добавить из новой архитектуры

Чего НЕ было в старом коде:
- ✅ **Streaming transcription** (чанки с overlap и fuzzy dedup)
- ✅ **Квантизированные модели** (q4_0: 120 MB вместо 466 MB)
- ✅ **Vulkan GPU offload** (через whisper.cpp build flag)
- ✅ **catch_unwind на JNI** (вместо crash на панике)
- ✅ **LocationAge enum** (Fresh/Recent/Stale с label в SMS)
- ✅ **Трёхуровневая стратегия моделей** (tiny для SOS, base для нормы, small для качества)
- ✅ **WorkManager** для SOS (переживает process death)
- ✅ **SOS all-contacts-fail handling** (алерт + UI + retry)

## 📊 Приоритеты

| Фаза | Что | Сложность | Срок |
|------|-----|-----------|------|
| **A** | Извлечь логику из ревью | Средняя | 1 день |
| **B** | Написать Rust-код по архитектуре | Высокая | 3-4 дня |
| **C** | Интегрировать с Kotlin UI | Средняя | 1-2 дня |
| **D** | Тестирование (эмулятор) | Средняя | 1-2 дня |

## 🎯 Критерий готовности

APK собирается, запускается на эмуляторе, транскрибирует аудио через Whisper Small q4_0, отправляет SOS-SMS (тестовый), не крашится.

---

*Подписано: Вари-Архивари, Добрый Робот*
