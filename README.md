# Benetto — VoiceNotes Local

[![Android](https://img.shields.io/badge/Android-3DDC84?logo=android&logoColor=white)](https://android.com)
[![Kotlin](https://img.shields.io/badge/Kotlin-7F52FF?logo=kotlin&logoColor=white)](https://kotlinlang.org)
[![Whisper](https://img.shields.io/badge/Whisper.cpp-OpenAI-412991)](https://github.com/ggerganov/whisper.cpp)
[![License](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)

**On-device voice transcription — no cloud, no API, everything offline.**

Record voice notes and transcribe them directly on your Android device using Whisper.cpp. Your voice never leaves your phone.

## ✨ Features

- 🎙️ **Record & Transcribe** — one tap to capture audio, automatic transcription
- 🔒 **100% Local** — Whisper Small (466 MB) runs on-device after first download, zero network requests for transcription
- 🌍 **Multilingual** — Russian, English, and 97+ languages via Whisper models
- ⚡ **Quality** — Whisper Small model for high accuracy speech recognition across 99 languages
- 📱 **Android 8+** — works on devices from 2017 onward
- 🎨 **Material Design** — clean, simple UI

## 📦 Download

[![Download APK](https://img.shields.io/badge/Download-APK-brightgreen)](https://всебудетхорошо.рф/benetto/app-release.apk)

*Current version: 1.0.0 (May 2026)*

## 🚀 Quick Start

```bash
# Clone
git clone https://github.com/570775-ux/Benetto.git
cd Benetto

# Build (requires Android SDK 36+)
export ANDROID_SDK_ROOT=~/android-sdk
./gradlew assembleRelease

# APK at app/build/outputs/apk/release/app-release.apk
```

## 🏗️ Architecture

```
┌─────────────────────────────────┐
│  VoiceNotes Local (Android)      │
│  ┌───────────┐  ┌─────────────┐ │
│  │ Recording │  │ Transcription│ │
│  │ Module    │→ │ Engine       │ │
│  │ (AudioRec)│  │ (Whisper.cpp)│ │
│  └───────────┘  └─────────────┘ │
│         ↓              ↓         │
│  ┌─────────────────────────────┐│
│  │     Room Database           ││
│  │  (notes, transcripts, meta) ││
│  └─────────────────────────────┘│
│         ↓                        │
│  ┌─────────────────────────────┐│
│  │     Jetpack Compose UI      ││
│  └─────────────────────────────┘│
└─────────────────────────────────┘
```

- **Audio Processing:** Custom WAV encoder, optimized for Whisper input format (16kHz, mono, 16-bit PCM)
- **Transcription Engine:** Whisper.cpp via JNI, Small model (~500MB), ~2-3x real-time
- **Data Layer:** Room + Repository pattern, offline-first
- **UI:** Jetpack Compose, Material 3

## 📊 Tech Stack

| Layer | Technology |
|-------|-----------|
| UI | Jetpack Compose, Material 3 |
| Architecture | MVVM + Repository Pattern |
| Database | Room (SQLite) |
| Audio | Android MediaRecorder + Custom WAV processor |
| Transcription | Whisper.cpp (C++ via JNI) |
| DI | Manual (lightweight, no framework) |
| Build | Gradle (Kotlin DSL) |

## 🗺️ Roadmap

See [ROADMAP.md](ROADMAP.md) for planned features and milestones.

## 🤝 Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## 📄 License

MIT License — see [LICENSE](LICENSE) for details.

## 🔗 Links

- 🌐 [Download APK](https://всебудетхорошо.рф/benetto/app-release.apk)
- 📖 [Documentation](https://всебудетхорошо.рф/benetto/)
- 🐛 [Issue Tracker](https://github.com/570775-ux/Benetto/issues)

---

*Built with ❤️ for privacy. No cloud. No tracking. Just your voice, your device.*
