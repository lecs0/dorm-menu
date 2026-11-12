# Source inspection — 2026-11-11

The public page and both supplied PDFs returned 200 over verified HTTPS. HTTP also returned 200, without redirecting to HTTPS; the implementation explicitly uses HTTPS and upgrades municipality HTTP links. It never disables certificate verification or falls back to HTTP.

Downloaded originals are in `fixtures/source/`. They are real municipality data, not synthetic fixtures. The HTML may contain irrelevant site scripts; it is stored for parsing tests, not served to browsers. Test strings explicitly marked synthetic are constructed inputs.

## Section boundary

The target `strong` heading is inside the **same paragraph as Jelky's menus**. Entries are separated by `br` elements, not standalone list items. Their labels include text outside the anchor; week 46's anchor ends in `men`, and the `ü` follows it. Collecting anchors alone loses that distinction and unpublished entries. The next `strong` heading is `KÖZÉPISKOLAI DIÉTÁS ÉTLAPOK:`. The scraper starts at the unique normalized target heading and stops at the next nonempty section heading. Missing or duplicate sections, unknown rows, multiple links, and untrusted links reject the update.

| Week | Page dates | Discovered relative link |
|---|---|---|
| 45 | 11.02–11.06 | `/images/2026/10/28/Kollegium202645het.pdf` |
| 46 | 11.09–11.13 | `/images/2026/11/04/Kollegium202646het.pdf` |
| 47 | 11.16–11.20 | No link |
| 48 | 11.23–11.27 | No link |

No URL directories are guessed. Filename year/week are supplementary evidence, cross-checked against page dates and ISO week. Filename years may refer to the calendar year or ISO week year, which can differ at New Year; both are checked rather than assuming equality. For unlinked rows the nearest unique year within 180 days is recorded as inferred evidence. If the year cannot be resolved uniquely, reject the update. Old page snapshots should be tested with their historical reference date.

## PDF findings

Both are one-page A4 landscape PDFs produced by Word 2019, with selectable meal-table text. Their graphical headers do **not** appear in Poppler's extracted text. Rendering both PDFs revealed:

- Week 45: `Kollégium`, `2026. november 2–6-ig`, `45. hét`.
- Week 46: `KOLLÉGIUM - 5. th.`, `2026.`, `november 9 - november 13-ig`, `46. hét`.

Those visually agree with the page and filename. Neither names Radnóti/III. Béla explicitly in the extractable table; attribution comes from the exact page section. Graphical header changes cannot currently be checked automatically. Every menu carries this limitation in `notes`; this is not an independent automatic institution/date validation. A future layout with extractable dates should get its own validation adapter. OCR of the decorative headers would add complexity and uncertain recognition; it is unnecessary for the selectable meal contents.

Columns are HÉTFŐ through PÉNTEK. Rows are REGGELI, EBÉD, VACSORA, each followed by calculated salt content. Week 46 spells breakfast `REGGELUI`. Friday dinner is blank in both. Meal names and numeric codes wrap across lines. Salt values use comma decimals and `g` (sometimes `2g`). Decorative footer text is graphical too.

Codes such as `1,3,7,` and `1,11` are printed beside foods. No allergen legend was found on either page. We preserve codes without mapping numbers to allergen names or assuming absent codes mean safe. Visual lines are preserved as components; they are not guaranteed dish boundaries.

The bbox adapter derives columns from weekday header positions and meal bands from the three salt rows. It rejects a changed page count, missing weekday/meal headers, unexpected empty cells, or a date range incompatible with this five-day layout. Other layouts require another adapter, rather than guessed dates.

Week 45 is 282,198 bytes; week 46 is 372,665 bytes. Metadata modification dates are October 21 and November 5 respectively. Thus the URL directory date should not be treated as a confirmed document modification or validity date.

## Residual source trust

HTTPS authenticates transport to the municipality host. It does not establish that their menu is correct or that their site is uncompromised. PDFs remain untrusted input. Downloads have a fixed HTTPS hostname allowlist, bounded redirects, timeouts, content-type and signature checks, and a 5 MB cap. Poppler is run without a shell, with a 20-second timeout and a 2 MB text cap; container limits constrain resources. Keep Poppler patched. NAS kernels and container isolation still need operational verification.
