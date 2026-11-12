# Architecture

`shared` contains the versioned menu model and calendar lifecycle. `api` discovers links, downloads PDFs, extracts text through Poppler, validates the known layout, atomically stores JSON, and serves snapshots through Axum. `app` contains Rust cache/schema validation, offline date rollover, Unicode keyword matching, and reminder planning. The native Android shell calls that Rust library through one JSON JNI bridge.

The server has one scheduled writer, protected by an advisory directory lock. Public requests only read memory. Every refresh constructs a candidate store; a failure anywhere preserves all existing menus. Commits use a synced temporary file, rename, and parent-directory sync. Process crashes cannot leave half a JSON file. A malformed existing store stops startup instead of silently discarding it.

The check interval defaults to 24 hours, dropping to 12 hours when the last menu ends within two days and no published upcoming menu exists. Failures double the delay up to seven days, with up to five minutes of jitter. Dates use chrono-tz Europe/Budapest; intervals are elapsed durations, so DST does not require interpreting nonexistent local clock times. `SIGUSR1` requests a local immediate check; no remote admin endpoint exists. A check in progress is not duplicated.

Every linked PDF in the recent/future window is fetched on a check, including unchanged URLs. This deliberately uses a few small downloads daily instead of relying on possibly incorrect modification headers. SHA-256 identifies content versions. URLs and page dates identify discovery; stable menu IDs use validity start and week. An unchanged PDF preserves its original menu timestamps. A failed extraction cannot replace good data. Temporary extraction files are removed; startup overwrites a leftover file safely.

Current/upcoming/expired status is computed from actual calendar ranges on every request. A future publication does not displace this week. The latest expired menu remains a fallback, including Saturday/Sunday after a Friday end. Storage keeps that fallback and current/future menus, with a 20-menu bound. Unknown validity is represented in the model, but the current source adapter rejects ambiguous rows rather than creating dated menus from them.

## Mobile choice

Compared against current official documentation:

| Choice | Rust/UI approach | Android considerations |
|---|---|---|
| [Dioxus](https://dioxuslabs.com/learn/0.7/guides/platforms/mobile/) | Rust UI and state, mobile WebView | Good portability; Android SDK/NDK and framework-specific tooling; native credential/notification integrations still need evaluation |
| [Slint](https://docs.slint.dev/latest/docs/slint/guide/platforms/mobile/android/) | Rust logic and declarative UI, rendered toolkit | Android target available; custom UI and native integrations; review its licensing for the intended distribution |
| [Tauri 2](https://v2.tauri.app/start/prerequisites/) | Rust backend plus web frontend | Android tooling supported; more frontend/IPC infrastructure than this small screen needs |
| Native Android Views + Rust JNI (selected) | Rust domain logic; Kotlin platform shell | Standard Android accessibility, Keystore, notifications and HTTP; modest Kotlin UI, SDK/NDK build required; less UI portability |

The selected first version favors straightforward platform integrations over an all-Rust UI. UI, network, preferences and Android services use Kotlin; menu lifecycle, schema handling, matching and reminder plans are Rust. The server is Rust throughout. This is not an all-Rust app. A Rust UI can replace the shell later without changing the domain model. No framework eliminates the need to test permissions, background behavior and secure storage on a phone.

No server preferences or accounts are stored. Tokens are separate per device, with SHA-256 verifiers on the server and constant-time verifier comparisons. Fast hashing is appropriate for uniformly random 256-bit tokens, not user passwords. A stolen valid token can be shared; token limits reduce abuse but do not prevent it.
