# Troubleshooting

## Source connectivity

Inspection initially failed because the execution sandbox could not resolve the host; the same requests succeeded after network permission. A future failure may have a different cause. On the laptop or NAS:

```sh
curl -I --connect-timeout 10 --max-time 30 'https://bajaionkormanyzat.hu/index.php/kozerdeku/szocialis-intezmenyek/intezmenyi-es-szunidei-gyermeketkeztetes'
curl --fail --connect-timeout 10 --max-time 45 --max-filesize 5000000 -o menu-page.html 'https://bajaionkormanyzat.hu/index.php/kozerdeku/szocialis-intezmenyek/intezmenyi-es-szunidei-gyermeketkeztetes'
pdftotext -layout downloaded-menu.pdf -
```

Check DNS, the system clock, CA certificates, connectivity and HTTP status. Don't disable TLS verification. For 403/access restrictions, stop automated retries and inspect the normally accessible page in your browser; provide saved HTML/PDFs if needed. No protection bypass is required. `cargo run -p dorm-api -- inspect menu-page.html` tests discovery without downloading.

If only HTTP were available later, the implementation currently fails closed. Do not silently downgrade. HTTP would allow interception/alteration of source downloads; public API HTTPS would not fix that. Supporting it would need an explicit reviewed configuration retaining host/redirect/type/signature/size checks and a visible source warning.

## Parse or layout errors

The writer rejects unexpected layouts and retains the last valid JSON. Check logs and compare saved HTML with the inspected fixture. Render new PDFs, not only extracted text: decorative date headers are absent from text extraction. For a new layout, add a separate adapter and regression fixture; do not remove validation until arbitrary PDF text is accepted. The current adapter only accepts the observed five-day layout, with blank Friday dinner. Date ambiguities reject the refresh and are logged.

Missing `pdftotext` requires Poppler utilities in PATH. Empty text may mean a scanned document or malformed PDF. Full OCR would be heavier and less predictable on the 512 MB NAS; prefer obtaining a text PDF from the publisher or processing OCR on a stronger trusted device. Never manufacture menu contents.

## Server and storage

`another server/check owns this data directory`: send SIGUSR1 to the running process instead of launching another writer. Do not delete lock files while the process runs. Locks release automatically on exit.

First-start empty menus are expected until one valid update completes. A source error keeps cached menus and marks source status error. A malformed persisted JSON stops startup: restore a known-good backup or move the bad file aside for inspection before an intentional fresh start.

401: check the token and verifier JSON, file permissions and revocation. Never paste the token into logs or URLs. 429: wait for the next minute bucket; reduce client refresh frequency. 304: use the cached response. Old menus with expired status: the dates really ended; a future menu does not extend them.

OOM/exit 137: inspect container memory and Poppler limits, reduce competing NAS workload, and consider extraction on a stronger device. `exec format error`: wrong CPU architecture. `illegal instruction`, syscall/seccomp errors or a container that fails before startup require checking CPU features, kernel and Docker compatibility rather than claiming ARMv7 alone is sufficient.
