# Dorm menu

Rust API and Android client for the Radnóti/III. Béla menus on Baja's municipality website. Built from the real week 45/46 PDFs inspected on 2026-11-11. HTTPS is available and used.

The server discovers only the intended section, preserves unlinked future entries, validates the inspected PDF layout, and retains good data on failures. The Android source uses a Rust domain core and a native Kotlin shell with offline cache, encrypted tokens, keyword highlights and optional local reminders.

## Run on Linux

Install a current Rust toolchain and Poppler (`pdftotext`; Debian/Ubuntu package `poppler-utils`). The dependency lockfile is included.

```sh
cargo test --workspace --locked
cargo build --release -p dorm-api --locked
mkdir -p data
chmod 700 data
cargo run -p dorm-api -- token
```

The last command prints a new random token and its SHA-256 verifier. Give the token to one user/device. Put only its verifier into `data/tokens.json`:

```json
{"my-phone":"REPLACE_WITH_PRINTED_SHA256_VERIFIER"}
```

```sh
chmod 600 data/tokens.json
cargo run -p dorm-api -- check
RUST_LOG=info cargo run -p dorm-api
```

The API binds `127.0.0.1:3000` by default and refreshes independently at startup and on its schedule. `check` is for when the server is stopped; while running, signal its process with `kill -USR1 SERVER_PID`. Configuration is by `DATA_DIR`, `TOKEN_FILE`, `BIND`, `CHECK_INTERVAL_SECONDS` (minimum one hour), and `RATE_LIMIT_RPM`.

```sh
curl http://127.0.0.1:3000/health
# Enter your token interactively; don't put it into a committed script.
read -rs DORM_TOKEN
curl -H "Authorization: Bearer $DORM_TOKEN" http://127.0.0.1:3000/api/v1/menus
unset DORM_TOKEN
```

Public use requires HTTPS through a tunnel/reverse proxy. Never expose the NAS administration interface. Rotate/revoke a device by atomically replacing its verifier in the token file; the API rereads it for every authenticated request.

The tested ARMv7 bundle is available locally at `target/dorm-menu-nas-bundle.tar.gz` (about 15 MB). It includes a private Poppler runtime and startup wrapper; production service/account setup is still required.

## Documentation

- [Real source inspection](docs/source-inspection.md)
- [Architecture and mobile framework comparison](docs/architecture.md)
- [API contract](docs/api.md)
- [NAS, ARMv7, Docker and Cloudflare deployment](docs/deployment.md)
- [Android build and behavior](docs/mobile-build.md)
- [Troubleshooting](docs/troubleshooting.md)
- [Stage completion and verification limits](docs/validation.md)

The PDF headers contain graphical dates that text extraction omits. Both supplied headers were visually cross-checked; automatic future-header date verification is not implemented. Allergen codes have no legend in these samples and remain uninterpreted. The static ARMv7 API and private Poppler bundle passed a live NAS test, using about 3.6 MB API resident memory afterward. A signed ARM64 Android debug APK was built and verified at `app/android/app/build/outputs/apk/debug/app-debug.apk` (about 1.6 MB). Phone installation, Docker deployment and tunnel connectivity remain unverified. See the validation report for the exact scope.
