# Kollégiumi menü

A Hungarian dormitory menu app for Android, backed by a self-hosted Rust API. It reads the published Radnóti / III. Béla dormitory menu PDFs from the Baja municipality website and makes the meals easier to browse on a phone.

**[Download the current Android test build](https://koliapp.lecso.me/)** · [API documentation](docs/api.md) · [Android build guide](docs/mobile-build.md)

## Features

- Browse the current and upcoming weekly menus by day and meal.
- Highlight selected keywords and source allergen codes, and set optional reminders for tomorrow's meals.
- Keep the last downloaded menu available offline; store the device token using Android Keystore.
- Automatically discover and parse source PDFs, retain the last validated menu on failure, and retry failed updates.
- Expose a token-protected API and a small Android download page.
- Run the server on Linux, including the tested ARMv7 NAS deployment.

The app displays allergen codes as published. The source PDFs inspected for this project did not provide a verified legend, so the app does not assign meanings to the codes. Confirm dietary requirements with the dormitory.

## Getting started

The download is the current **debug/test APK** for ARM64 devices running Android 8.0 or newer. It is not a production-signed release. The running server requires a device access token; contact the app maintainer for one. Never commit tokens, signing keys, Cloudflare credentials, or local configuration.

To run your own API on Linux, install Rust and Poppler (`pdftotext`, provided by `poppler-utils` on Debian/Ubuntu):

```sh
cargo test --workspace --locked
cargo build --release -p dorm-api --locked
mkdir -p data
chmod 700 data
cargo run -p dorm-api -- token
```

The last command generates a device token and its SHA-256 verifier. Keep the token private and place only its verifier in `data/tokens.json`:

```json
{"my-phone":"REPLACE_WITH_PRINTED_SHA256_VERIFIER"}
```

```sh
chmod 600 data/tokens.json
cargo run -p dorm-api -- check
RUST_LOG=info cargo run -p dorm-api
```

The API listens on `127.0.0.1:3000` by default. To make it available to phones, use HTTPS with a reverse proxy or tunnel and keep the health endpoint and NAS administration private. See [API documentation](docs/api.md) and [deployment notes](docs/deployment.md) for configuration, authentication, and deployment details.

The download page and APK in `website/dist/` are embedded in the API build. Updating the Android APK on the website requires replacing that file and rebuilding and redeploying the API.

## Project layout

- `api/` — Rust scraper, PDF parser, authenticated API, and download routes.
- `shared/` — shared menu data structures.
- `app/` — Rust mobile core and native Android interface.
- `website/` — download page and the current test APK.
- `fixtures/` — inspected source-page and menu examples used by tests.
- `deploy/`, `scripts/`, and `docs/` — deployment helpers, build scripts, and technical documentation.

## Development and limitations

See [Android build and behavior](docs/mobile-build.md), [architecture](docs/architecture.md), [source inspection](docs/source-inspection.md), and [validation](docs/validation.md). The PDFs use graphical header dates that text extraction cannot verify automatically; the included examples were checked visually. PDF layouts and the source website may change and require parser updates.

The Android application and website are credited to **Lecs0**.
