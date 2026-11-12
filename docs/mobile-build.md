# Android client

The Android project is in `app/android`. The Rust `dorm-app` library exposes a single JNI entry point. Native Android Views build the screen; this avoids a WebView and gives direct access to Android platform storage and notifications. See the comparison in `architecture.md`.

## Linux prerequisites

Install JDK 17, Android SDK platform 35, platform-tools, Android NDK 28.2.13676358 and Gradle 8.11.1 (for the pinned Android Gradle Plugin 8.9.1). Set `JAVA_HOME`, `ANDROID_HOME` and `ANDROID_NDK_HOME` to your installations. See [Android Gradle Plugin 8.9 compatibility](https://developer.android.com/build/releases/past-releases/agp-8-9-0-release-notes) and [Android SDK tools](https://developer.android.com/tools). The repo does not include a Gradle wrapper binary; use the specified Gradle version rather than an arbitrary system version.

```sh
rustup target add aarch64-linux-android
cargo install cargo-ndk --locked
./scripts/build-android.sh
adb install -r app/android/app/build/outputs/apk/debug/app-debug.apk
```

The build script targets arm64 phones. For an ARMv7 phone, install Rust's `armv7-linux-androideabi` target and add `-t armeabi-v7a` to cargo-ndk. That target is unrelated to the NAS Linux target. For an emulator, build the matching ABI too. Release distribution requires a private Android signing key and release signing configuration; no key is included. The minimum Android version is 8.0 (API 26).

The Rust library linked for Android arm64 with NDK 28.2.13676358; the Kotlin UI compiled and Gradle assembled `app/build/outputs/apk/debug/app-debug.apk` (about 1.6 MB). Its APK v2 signature and 16 KB ZIP alignment verified. The APK was subsequently installed and tested on a Samsung Galaxy Note20 running Android 13: real menus, day selection, keyword highlighting, offline cache, encrypted token persistence, revocation handling and one reminder notification passed. Background/Doze and reboot reliability remain unverified. The artifact is a debug build, not a production-signed release.

## First use

Open Preferences and enter your public HTTPS API base address and a separately generated device token. Save, then refresh. Tokens are AES-GCM encrypted using a non-exportable Android Keystore key; only ciphertext and IV are stored in private preferences. Android backups are disabled. A rooted/compromised app process can still use the key and access plaintext tokens, as explained in the [Keystore documentation](https://developer.android.com/privacy-and-security/keystore). Changing endpoints clears the old cache and ETag to avoid mixing sources.

Menus load from private local cache immediately. Switching days or weeks makes no network requests. Automatic checks run on app open after the configured interval (default six hours); manual refresh is available. Requests use ETags, bounded responses and timeouts; redirects are refused to protect credentials. Failed sync keeps the previous cache. Failures use an exponential retry gate on later app opens; there is no polling service. Invalid/revoked tokens are shown clearly. Unsupported cache schemas are rejected; clearing cache is available in Preferences.

Rust recomputes current/upcoming/expired status using the current Budapest calendar date even offline, rather than trusting the cached server status. Friday validity ends on Friday. On weekends the old menu is visibly expired and the published next menu can be selected. Unlinked page entries are shown as not published. Last source-update time and original PDF links are visible.

Keyword preferences are edited as comma-separated values; removing one deletes it. Matching normalizes Unicode to NFC, lowercases it and uses literal substrings. Accents remain meaningful: `turo` does not match `túró`. `hal` also matches `halászlé`. Matching is a personal reminder aid, not a reliable allergen detector. Raw source allergen codes are displayed but there is no allergen picker because their meaning/completeness has not been established.

## Reminder limits

The Rust core plans reminders for tomorrow's matching meals in cached data. The Android shell schedules them at the selected Europe/Budapest time, using inexact alarms and stable date/category IDs. Refreshing or editing preferences cancels/recomputes pending reminders. Delivered IDs prevent duplicates and are bounded to the most recent 100 IDs. Notifications ask users to check whether they need to cancel an order; nothing cancels an order automatically.

Android 13+ requires notification permission. Android may delay inexact alarms during idle/battery restrictions; no exact-alarm permission is requested. See [Android alarm behavior](https://developer.android.com/develop/background-work/services/alarms). Delayed reminders are discarded if their intended notification date has passed. On reboot or clock/timezone changes, alarms are canceled; reopen the app to recompute them. The UI explains that reminders use cached data and require opening the app to sync and refresh plans. This deliberately limited first version has no background menu sync, push, WorkManager worker, or promise of daily reminders without opening the app. A published menu cannot be discovered immediately while the app is closed.

Device verification still needs: Keystore persistence/recovery, denied notification permission, alarms under Doze, timezone/DST change, reboot behavior, endpoint/token changes, screen reader support, small-screen layout, offline startup and TLS errors.

## Private phone test with Tailscale

Tailscale on the phone and API host can provide private connectivity without Cloudflare. Keep the API bound to loopback and put Tailscale Serve HTTPS in front of it. Use the host's full `.ts.net` HTTPS hostname and Serve port in the app; a raw `100.x` address does not match the HTTPS certificate. See [Tailscale Serve](https://tailscale.com/docs/features/tailscale-serve).

For a temporary laptop test, after starting the API:

```sh
tailscale serve --https=8443 http://127.0.0.1:3000
```

Run Serve in the foreground so Ctrl+C stops the temporary share. Check `tailscale serve status` first and avoid replacing another service's configuration. Tailscale may require an owner/admin web consent step to enable Serve/HTTPS. Certificate hostnames appear in public certificate-transparency logs, while Serve access remains tailnet-only. See [Tailscale HTTPS setup](https://tailscale.com/docs/how-to/set-up-https-certificates). The app still uses its own bearer token; no Tailscale identity headers are trusted by the API. Tailscale ACLs must permit the phone to reach the selected port.

Copy the debug APK from `app/android/app/build/outputs/apk/debug/app-debug.apk` to the phone (for example using Tailscale file transfer or USB), open it, and allow installation from the chosen file app if Android asks. This APK requires ARM64 Android 8.0 or newer; other ABIs require a matching native-library build. In Preferences enter the Serve HTTPS base URL and a generated device token, then refresh. Release builds reject HTTP endpoint URLs. For authorized USB testing, debug builds allow exactly `http://127.0.0.1` with `adb reverse tcp:3000 tcp:3000`; the debug network policy denies cleartext to every other host.

Check current and upcoming/unpublished menus, today's selected day, source links, and the source-update time. Turn off phone connectivity and reopen the app to check cached menus and the offline state, then restore connectivity and refresh. Enter a known matching keyword to check highlights. For notification testing choose a keyword present in tomorrow's actual menu, enable reminders and notification permission, and choose a time a few minutes ahead in Europe/Budapest. Inexact Android alarms may be delayed. Revoking a test token must make the next network refresh show a token error while leaving the already cached menu available.

The NAS test process was stopped and removed. A phone cannot use the NAS's address until a service has been deployed there again. The laptop test endpoint is separate from the NAS address. No phone installation or notification delivery is claimed until it has actually been observed on the user's device.
