# Build a Rust Dorm Menu API and Android App

I want you to help me design and build a dormitory menu application consisting of:

1. A lightweight Rust API that runs on my NAS.
2. A scraper that finds my dorm's weekly menu PDFs on the municipality website.
3. PDF text extraction and parsing into structured JSON.
4. An Android app written in Rust as much as practical.
5. Deployment and maintenance documentation.

The primary goal is to make my dorm's menu easy to view on my phone. Food preferences, allergen information, and reminders are secondary features that should be added after the menu reliably works.

I am an IT student. Explain important decisions clearly, but you can use technical terminology and terminal commands.

## My devices and environment

### Development laptop

- Linux laptop, currently used for development.
- I plan to use Codex on the laptop.
- I can provide additional details about the Linux distribution and hardware if needed.

### NAS

- **Model:** ASUSTOR AS1002T v2
- **Operating system:** ADM 4.0.7
- **Processor:** 32-bit ARMv7
- **Memory:** 512 MB
- **Docker:** Installed and working, but the NAS and its Docker environment are relatively old.
- **Expected traffic:** Approximately 20 API requests per day.
- **Domain:** I own a domain managed by Cloudflare.
- **External access:** I would prefer Cloudflare Tunnel if it can run reliably in my environment.

The NAS is resource-constrained. The API should use little memory and nearly no CPU while idle. Briefly using more CPU when processing a PDF is acceptable.

Do not assume current Docker images, Rust dependencies, or Cloudflare Tunnel binaries support 32-bit ARMv7. Verify compatibility where possible and clearly mark anything that has not been tested.

## Actual menu source

The menu page is:

```text
http://bajaionkormanyzat.hu/index.php/kozerdeku/szocialis-intezmenyek/intezmenyi-es-szunidei-gyermeketkeztetes
```

We only need menus from this section:

```text
RADNÓTI MIKLÓS KOLLÉGIUMI ÉS III. BÉLA GIMNÁZIUMI ÉTLAPOK:
```

The page also contains menus for other institutions. **Do not mix those menus with the Radnóti/III. Béla menus.**

Examples from this section:

```text
45. heti menü (11.02. - 11.06.)
http://bajaionkormanyzat.hu/images/2026/10/28/Kollegium202645het.pdf

46. heti menü (11.09. - 11.13.)
http://bajaionkormanyzat.hu/images/2026/11/04/Kollegium202646het.pdf

47. heti menü (11.16. - 11.20.)
Link may not be published yet.

48. heti menü (11.23. - 11.27.)
Link may not be published yet.
```

At the time I provided this information, the current menu was week 46:

```text
http://bajaionkormanyzat.hu/images/2026/11/04/Kollegium202646het.pdf
```

**Important:** Week 46 is an example of the menu that was current when I wrote this prompt. Do not hardcode it as the permanently current menu. Determine the current and upcoming menus from their dates.

The URL contains a date such as `/images/2026/11/04/`. It might be an upload date, but that is not confirmed and I do not know it in advance. **Do not construct PDF URLs by guessing this date.** Discover links from the relevant section of the main page.

The example filenames appear to contain a year and week number, such as `Kollegium202646het.pdf`, but treat that as a useful clue rather than a guaranteed permanent format.

The sample menu appears to cover Monday through Friday. Do not assume the same date range or number of days will always apply; inspect the published dates and the PDF.

## First: inspect the real website and PDFs

Before deciding how to parse the menu:

1. Fetch the main page and inspect its HTML.
2. Find the heading for **RADNÓTI MIKLÓS KOLLÉGIUMI ÉS III. BÉLA GIMNÁZIUMI ÉTLAPOK**.
3. Determine how the menu entries and links are structured beneath it.
4. Determine how to stop collecting links when the next unrelated section starts.
5. Inspect the week 45 and week 46 PDFs.
6. Check whether their text is selectable/extractable or whether the PDFs contain scanned images.
7. Compare the page dates, PDF contents, and filenames.
8. Identify the structure of each meal and any allergen information.
9. Check whether menus include breakfast, lunch, dinner, or other categories.

Do not invent details about the PDF layout or allergen format.

If the website returns a **403, timeout, or other error**, investigate using normal, respectful diagnostics. Explain the problem and show me how to check whether the page opens from my laptop or NAS. Ask me for a downloaded PDF or saved HTML if necessary. Do not try to bypass access controls or security protections.

If you cannot access the source at first, continue with source-independent work using **clearly labeled sample data**, and isolate the website-specific scraper and PDF parser so the real implementation can be completed later.

## Important: the source uses HTTP

The URLs I provided use `http://`.

Test whether the page and PDFs also work over HTTPS. If HTTPS works reliably, prefer it.

If the source is only available over HTTP:

- Explain that traffic between the NAS and the municipality website is not encrypted or authenticated.
- Do not suggest that HTTPS on my public API protects the original HTTP download.
- Validate the expected source host, redirects, content type or file signature, and file size.
- Treat downloaded PDFs as untrusted input.
- Explain the remaining risk of altered source content.
- Allow users to see the original source link and the last successful update time.

Do not disable TLS verification to make HTTPS work.

# Part 1: Scraper

## Finding the correct PDF links

The scraper should:

- Fetch the municipality page on a configurable schedule.
- Locate the **Radnóti Miklós Kollégium and III. Béla Gimnázium** section.
- Extract the menu entries associated with that section only.
- Recognize entries that exist as text but do not yet have a PDF link.
- Read the displayed week number and date range when available.
- Resolve relative links correctly.
- Accept PDF links only from the expected host or another explicitly configured trusted host.
- Handle Hungarian text and accented characters correctly.
- Avoid depending on one exact link label if a reasonable alternative is possible.
- Detect when the page layout changes and report a useful error.
- Never replace valid data with unrelated menus.

Do not scrape the website every time a mobile user opens the app. The scraper must run independently of public API requests.

Use reasonable timeouts, a meaningful User-Agent, bounded response sizes, and polite request frequency.

## Detecting new and updated PDFs

The scraper should detect a new PDF based on the page link and associated dates. It should also handle an existing PDF URL being replaced with new content. Depending on what the server supports, use a combination of the page entry and URL, HTTP modification headers, a hash of the downloaded PDF, and last successful check time.

Do not rely exclusively on the PDF URL changing. Do not guess URL directory dates or generate future URLs from the week number.

## Current and upcoming menus

The website may list both the current week and one or more future weeks. The API should support this properly.

- On Wednesday, the page could contain a valid menu for this week and a published menu for next week.
- The app should show **this week's menu** first.
- The user should be able to view the **next week's menu** once it is available.
- A newly published future menu must not replace the current one prematurely.
- When the new week begins, the previously upcoming menu should become current automatically.
- If a future entry exists on the page but has no PDF link, show it as **not yet published** if that can be determined reliably.

Use the actual date range published on the page and/or in the PDF. If the sources disagree, do not silently choose a date: flag the discrepancy and preserve the original source information.

Prefer explicit calendar dates over assumptions such as “the menu is always valid for seven days.”

## Check schedule

Make the schedule configurable. A sensible starting point could be:

- Check approximately once per day.
- Check somewhat more often near the end of the current menu period if the next menu has not appeared.
- Use backoff if the website is unavailable.
- Add a small amount of randomness if appropriate.

Choose a schedule based on what you learn from the real page. Keep requests low; this app will serve only a small number of users.

Use the `Europe/Budapest` time zone for menu dates and schedule decisions. Handle daylight-saving changes correctly.

Provide a secure local way for me to trigger an immediate source check for testing and administration.

# Part 2: PDF extraction and parsing

## PDF extraction

Choose a lightweight extraction method that is compatible with my NAS.

First check whether the PDFs have extractable text. If they are scanned images:

- Explain why normal text extraction will not work.
- Evaluate whether OCR is realistic on the NAS.
- Suggest a simpler alternative if OCR would be too heavy or unreliable.

If using external tools such as a PDF text extractor, document required packages, ARMv7 compatibility, how to run them in Docker, timeouts and resource limits, and handling of malformed or unusually large PDFs.

Do not assume the PDF layout based only on the filename.

## Parsing the menu

After inspecting real samples, parse the menu into structured data where available:

- Institution or menu name.
- Week number.
- Start and end dates.
- Individual dates and day names.
- Meal categories such as breakfast, lunch, and dinner.
- Dishes or components.
- Allergen information, if actually published.
- Notes and other useful information.
- Source page and PDF URLs.
- Retrieval and update timestamps.

Preserve the original Hungarian text and characters.

Make the PDF parsing logic modular so it can be adjusted if the dorm changes the layout.

Use the two provided PDFs as representative samples once they can be accessed. If more samples are needed to understand layout variations, tell me what would be useful.

## Validation and safe updates

Before accepting a newly parsed menu:

- Confirm that it appears to belong to the intended institution.
- Check whether dates and meal contents were extracted plausibly.
- Check for missing or suspiciously small amounts of text.
- Compare relevant dates with the page entry if available.
- Flag ambiguity rather than guessing.

Only replace stored menu data after a new version has passed validation.

Keep the last successfully parsed menu if the source website is unavailable, the PDF download or extraction fails, parsing produces incomplete or suspicious data, or the website layout changes.

Use atomic writes or another appropriate method to prevent corrupted files after a crash or NAS restart.

## Allergen information

Never invent allergen information from food names.

- Treat allergens as **confirmed** only when explicitly supported by the published source and reliably parsed.
- If allergen data is missing or ambiguous, mark it as **unknown**.
- Preserve original allergen labels or codes where useful.
- In the app, explain that allergen data may be incomplete and that users with serious allergies should verify it with the dorm or original source.

# Part 3: Menu lifetime and storage

My original idea was to delete a menu after a week. Instead, implement a more reliable lifecycle:

- Keep the current valid menu.
- Keep the next published menu when available.
- Mark a menu as expired once its actual date range has passed.
- If the next menu is late, keep showing the last valid menu as **expired**, with a clear warning.
- If validity dates cannot be determined, show them as **unknown** rather than guessing.
- Remove old temporary files.
- Keep storage use small.
- A long-term archive is optional and not needed for the first version.

If a menu covers only Monday through Friday, don't automatically assume it remains valid through Sunday. The app can still show it as the last available menu on the weekend and clearly show its actual dates.

Use simple JSON files or another lightweight storage solution unless there is a strong reason to use a database. A separate SQL server is unnecessary for the expected traffic.

# Part 4: Rust API

Build the API in Rust with a lightweight, well-maintained framework.

Suggested endpoints:

```http
GET /health
GET /api/v1/menus
GET /api/v1/menus/current
GET /api/v1/menus/next
```

Adjust these endpoints if there is a good reason, but keep the API small.

A response might look like this:

```json
{
  "schema_version": 1,
  "last_source_check": "2026-11-11T10:00:00Z",
  "current_menu": {
    "menu_id": "2026-week-46-example",
    "status": "current",
    "week_number": 46,
    "valid_from": "2026-11-09",
    "valid_until": "2026-11-13",
    "source_url": "http://bajaionkormanyzat.hu/images/2026/11/04/Kollegium202646het.pdf",
    "days": []
  },
  "next_menu": null
}
```

**This JSON is only an example.** Design the actual schema after inspecting the real PDF. Do not invent meal contents.

The API should:

- Return consistent JSON and useful HTTP status codes.
- Clearly distinguish current, upcoming, expired, unavailable, and unknown-validity data.
- Handle a first start when no menu has been processed.
- Use stable menu identifiers or version hashes.
- Support ETags or a similar conditional request mechanism.
- Avoid exposing secrets, stack traces, internal paths, or NAS information.
- Use useful structured logs without logging credentials.
- Limit request and response sizes.
- Explain whether the health endpoint is public or private.

Version the API schema so changes can be handled safely by the app.

# Part 5: Authentication and security

I want to give selected people a token to access the API. I do not need a full account system.

Implement practical token-based authentication:

- Public traffic must use HTTPS.
- Require authentication for menu endpoints.
- Send tokens using the `Authorization: Bearer <token>` header.
- Do not put tokens in URLs.
- Do not hardcode a shared permanent token in the app.
- Let users enter a token manually in the app.
- Prefer separate tokens per user or device if practical.
- Allow tokens to be revoked or rotated without rebuilding the app.
- Generate strong random tokens.
- Store server-side verification material securely.
- Use appropriate constant-time comparison for secrets.
- Do not log tokens.
- Store tokens using secure Android storage where practical.
- Explain the limitations of token storage on rooted or compromised devices.

Explain that a token does not completely prevent abuse: someone with a valid token could share it or send too many requests.

Add lightweight abuse protection: configurable rate limiting, request size limits, sensible timeouts, Cloudflare protection where suitable, and safe handling of client IPs behind a proxy; do not trust arbitrary forwarded headers.

The API must not expose arbitrary file access, command execution, NAS administration, or user-supplied scraper URLs.

# Part 6: Cloudflare and NAS deployment

I want to access the API through my domain, preferably using Cloudflare Tunnel.

1. Verify whether Cloudflare Tunnel can run on my ARMv7 NAS and ADM/Docker version.
2. If it can, explain the setup.
3. Keep the API private behind the tunnel rather than exposing the NAS management interface.
4. If it cannot run on the NAS, propose realistic alternatives, such as running the tunnel on another always-on device.
5. Explain any relevant costs or external dependencies without assuming a specific feature is free.
6. Explain how API authentication works alongside Cloudflare.
7. If recommending Cloudflare Access, explain how the Android app will authenticate and whether it adds an extra login or credential flow.

Do not claim compatibility is tested unless it really is.

# Part 7: Android app written in Rust

I use Android, so prioritize Android for the first version. Keep the design reasonably portable, but iOS support is not a requirement for the first release.

Compare realistic Rust mobile UI choices before selecting one. Consider Android support and maturity, UI quality, build and installation difficulty on Linux, offline storage, secure credential storage, notifications, maintenance and documentation, and whether native Android code is needed for certain features.

Recommend a practical framework and explain its limitations. The app should be written in Rust as much as practical, while allowing small platform-specific integrations where needed.

## Main menu screen

The app should show:

- Current week's menu and actual date range.
- Today's meals prominently when today falls within the menu.
- A way to select other available days.
- The next week's menu when published.
- A clear “not published yet” state when applicable.
- An **expired** warning if displaying an old menu.
- Last successful update time.
- A link to the original PDF.
- Highlights for matching disliked-food keywords.
- Relevant allergen information where actually available.

Keep the UI simple and readable on a phone.

On a weekend, show the last available menu with its actual dates and allow the user to view the upcoming menu if it exists.

## Preferences

Allow the user to:

- Enter and update their API token.
- Add disliked-food keywords, for example `szendvics`, `gomba`, and `hal`.
- Edit and delete keywords.
- Choose allergens if the source provides reliable allergen information.
- Enable or disable reminders.
- Choose a reminder time.
- Manually refresh the menu.
- View connection and sync status.
- Clear cached menu data.

Store preferences locally by default. No user accounts or server-side preference storage are needed initially.

Keyword matching should be case-insensitive and support Hungarian accented characters. Make its matching behavior predictable and explain that matching food names is not a reliable allergen detection system.

## Reminders

If a meal matches a disliked-food keyword, the app should optionally remind the user to check whether they need to cancel their meal order.

Example:

```text
Tomorrow's breakfast includes “szendvics”.
Check whether you need to cancel your order.
```

Requirements:

- Do not claim that an order was canceled.
- Do not attempt automatic cancellation.
- Allow reminders to be turned on or off.
- Allow the reminder time to be configured.
- Avoid duplicate notifications.
- Update or cancel scheduled reminders if the menu or preferences change.
- Explain Android notification permissions and background limitations.
- If automatic background syncing is unreliable in the chosen framework, implement a simpler reliable approach first and clearly explain the limitation.

# Part 8: Caching and request efficiency

The app should avoid unnecessary API requests:

- Cache current and upcoming menus locally.
- Display cached data immediately.
- Work offline with a clear indicator.
- Do not request data when merely switching screens.
- Use configurable automatic check intervals.
- Allow manual refresh.
- Use ETags or equivalent conditional requests.
- Retry failed requests with backoff.
- Avoid draining battery.
- Handle schema changes safely.
- Handle invalid or revoked tokens clearly.

Explain that without push notifications or a background check, the app cannot know immediately when a new menu has appeared.

# Part 9: Project structure and deployment

Use a clear structure, for example:

```text
dorm-menu/
├── api/
│   ├── src/
│   │   ├── main.rs
│   │   ├── config.rs
│   │   ├── api/
│   │   ├── auth/
│   │   ├── scraper/
│   │   ├── parser/
│   │   └── storage/
│   ├── tests/
│   ├── Cargo.toml
│   └── Dockerfile
├── app/
│   ├── src/
│   ├── assets/
│   └── Cargo.toml
├── shared/
│   ├── src/
│   └── Cargo.toml
├── deploy/
│   ├── docker-compose.yml
│   └── .env.example
├── docs/
│   ├── architecture.md
│   ├── api.md
│   ├── deployment.md
│   ├── mobile-build.md
│   └── troubleshooting.md
├── Cargo.toml
├── .gitignore
└── README.md
```

Change the layout if needed.

For Docker deployment:

- Keep the image reasonably small.
- Support ARMv7 if verified.
- Prefer building on the laptop or another stronger machine over compiling on the NAS.
- Consider cross-compilation.
- Use persistent storage for menus and configuration.
- Run as a non-root user where practical.
- Limit container privileges and writable paths.
- Configure automatic restart.
- Explain logs, updates, backup, and restore.
- Provide a `.env.example` with placeholders.
- Never commit actual tokens or credentials.

If the old NAS cannot run a selected dependency or container, identify the exact limitation and suggest an alternative.

# Part 10: Testing

Implement meaningful tests where appropriate, including:

- Finding only the Radnóti/III. Béla section on a representative HTML page.
- Handling menu entries without links.
- Discovering PDF links without guessing directory dates.
- Handling the same PDF URL with changed content.
- Parsing representative week 45 and week 46 samples.
- Hungarian text and accented characters.
- Date and week-number parsing, including year boundaries.
- Current versus upcoming menu selection.
- Weekend behavior.
- Missing or ambiguous dates.
- A failed extraction that does not replace valid data.
- API authentication and token revocation.
- Rate limits and conditional requests.
- Keyword matching.
- Offline caching.
- Reminder scheduling and duplicate prevention where practical.

Clearly distinguish synthetic fixtures from data extracted from the actual municipality PDFs.

Report what you tested and what you could not test. Do not claim Android, ARMv7, Cloudflare Tunnel, or NAS compatibility has been verified if it has not.

# Part 11: Development stages

Work in practical stages:

### Stage 1: Inspect the source

- Inspect the municipality page.
- Identify how the Radnóti/III. Béla section is structured.
- Inspect the two provided PDFs.
- Determine whether the PDF text can be extracted.
- Explain the actual layout and any obstacles.

### Stage 2: Build the scraper and parser

- Discover links from the correct section.
- Download and parse real menu data.
- Preserve current and upcoming menus separately.
- Validate parsed data.
- Save the results safely.

### Stage 3: Build the API

- Add menu endpoints.
- Add token authentication.
- Add caching support, rate limits, logs, and errors.
- Add scheduled source checks.

### Stage 4: Build the Android app

- First display the menu and cache it offline.
- Add token setup.
- Add current and upcoming menu navigation.
- Add food preferences and reminders after the core functionality works.

### Stage 5: Deploy

- Prepare and verify an ARMv7 build where possible.
- Create Docker configuration.
- Document Cloudflare and NAS setup.
- Test the application end to end as far as the available environment allows.

At the end of each stage, tell me what is complete, what was tested, and what still needs verification.

If the website is inaccessible from your environment, do not invent its structure. Tell me what happened and ask me to provide the PDF or saved HTML when needed. Continue useful independent work in the meantime.
