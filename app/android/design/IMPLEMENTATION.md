# Warm cafeteria Android redesign

Chosen direction: A, native Android Views, no added UI dependencies or food assets.

The Android shell now uses cream/forest-green and charcoal/sage palettes, rounded meal cards, a horizontal date strip, week selection, persistent bottom navigation, Hungarian copy, explicit missing/unpublished states, source timestamps, offline and stale indicators. Settings group keywords and reminders, use a native time picker, show notification permission state, and isolate connection/token editing. Source meal text and codes remain unchanged. All action targets are at least 48dp; content scrolls and text uses sp. There are no custom animations.

Rust/JNI and backend were not changed. AES-GCM Keystore storage, preferences keys, cache validation/atomic writes, conditional ETags, refresh interval/backoff, reminder IDs, duplicate prevention and inexact alarms are retained. Default connection is https://koliapp.lecso.me; existing connection settings are respected. Reminder messages are localized at the Android boundary.

## Verification

- assembleDebug succeeded using existing local JDK 17, SDK 35, Gradle 8.11.1 and cached dependencies.
- Installed as an update on Samsung SM-N980F using the previous debug signing key; no uninstall or data clearing.
- Actual cached menu, automatic selection of today, live authenticated refresh, preserved keyword preference, menu/settings navigation and both system themes inspected on device.
- Screenshots in verification/. These are actual phone captures; the phone uses its own system font.
- Android runtime error log checked, no errors observed during these checks.

## Remaining limits

This remains a debug APK. Doze/background delivery, reboot/DST and notification denial need longer device testing. Refresh still runs on app opening/manual request, as before; no new background worker. Revoked-token, malformed-cache and source-error branches were reviewed but not fault-injected against the production connection. TalkBack and other devices have not been tested. The app follows system theme; changing configuration can discard an unsaved settings form. Allergen meanings remain unknown; no inferred allergen selection was added.

Additional device checks passed: Thursday selection and saved `sajt` highlight; Friday's missing dinner message; unpublished week labels; 1.3× system font rendering. System night mode restored to yes and font scale restored to 1.0. Offline indicator code was reviewed; connectivity was not disabled on the user's phone.
