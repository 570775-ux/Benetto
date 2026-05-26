# Contributing to Benetto

Thanks for your interest! Benetto is an open-source, privacy-first voice transcription app.

## Getting Started

1. **Fork** the repository
2. **Clone** your fork
3. **Build** the project (see README Quick Start)
4. **Create** a feature branch: `git checkout -b feat/my-feature`
5. **Commit** your changes: `git commit -m "feat: description"`
6. **Push**: `git push origin feat/my-feature`
7. **Open a Pull Request**

## Commit Convention

We use [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` — new feature
- `fix:` — bug fix
- `docs:` — documentation
- `refactor:` — code restructuring
- `perf:` — performance improvement
- `test:` — tests
- `chore:` — build, CI, dependencies

## Code Style

- **Kotlin:** Follow [Android Kotlin Style Guide](https://developer.android.com/kotlin/style-guide)
- **C++ (JNI):** Google C++ Style Guide, `snake_case` for functions
- **Formatting:** Default Android Studio formatter (Ctrl+Alt+L)
- **Naming:** Descriptive names over brevity

## Architecture Rules

- **UI never talks to data layer directly** — always through ViewModel + Repository
- **All database operations on background thread** (Room handles this)
- **No hardcoded strings** — use `strings.xml`
- **Privacy first** — no analytics, no tracking, no network calls (unless explicitly user-initiated for optional features)

## Testing

```bash
# Unit tests
./gradlew test

# Instrumented tests (requires emulator/device)
./gradlew connectedAndroidTest
```

## Need Help?

- 🐛 **Bugs:** Open an [issue](https://github.com/570775-ux/Benetto/issues)
- 💡 **Feature requests:** Open an issue with `enhancement` label
- 💬 **Questions:** Start a [discussion](https://github.com/570775-ux/Benetto/discussions)

---

*Your voice, your device, your privacy. Let's build together.*
