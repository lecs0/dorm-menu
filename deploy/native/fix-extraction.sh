#!/bin/sh
set -eu
umask 077
[ "$(id -u)" = 0 ] || { echo 'Run with sudo.'; exit 1; }
base=/volume1/dorm-menu
native=$base/native
stage=/volume1/home/admin/dorm-menu-deploy
[ -f "$base/.managed-by-dorm-menu" ] || exit 1
report=$base/status/extraction-fix-report.txt
: > "$report"
chown 999:999 "$report"
chmod 600 "$report"
exec 3>&1
exec >> "$report" 2>&1
trap 'result=$?; tail -n 20 "$report" >&3; exit "$result"' EXIT
cd "$stage"
sha256sum -c ZLIB-SHA256SUMS
tar -xzf zlib-runtime.tar.gz -C "$native/api-root/app"
chown 0:0 "$native/api-root/app/runtime/lib/arm-linux-gnueabihf/libz.so.1.2.11"
chmod 644 "$native/api-root/app/runtime/lib/arm-linux-gnueabihf/libz.so.1.2.11"
cp "$stage/diagnostic.pdf" "$native/api-root/data/diagnostic.pdf"
chown 10001:10001 "$native/api-root/data/diagnostic.pdf"
chmod 600 "$native/api-root/data/diagnostic.pdf"
set +e
"$native/bin/dorm-launcher" api --pdf-check > "$native/run/diagnostic.bbox.html" 2> "$native/run/diagnostic.stderr"
result=$?
set -e
echo "PDF extraction exit status: $result"
cat "$native/run/diagnostic.stderr"
wc -c "$native/run/diagnostic.bbox.html"
rm -f "$native/api-root/data/diagnostic.pdf"
[ "$result" = 0 ] || exit 1
[ -s "$native/run/diagnostic.bbox.html" ] || exit 1
pid=$(cat "$native/run/api.child.pid")
case "$pid" in *[!0-9]*|'') exit 1;; esac
[ "$(readlink /proc/"$pid"/exe)" = "$native/api-root/app/dorm-api" ] || { echo 'API identity check failed'; exit 1; }
before=$(wc -l < "$native/logs/api.log")
kill -USR1 "$pid"
echo 'Requested live menu refresh...'
refreshed=no
for attempt in 1 2 3 4 5 6 7 8 9 10; do
 sleep 2
 sed -n "$((before + 1)),\$p" "$native/logs/api.log" > "$native/run/refresh-check.log"
 if grep -q 'source_update_ok' "$native/run/refresh-check.log"; then refreshed=yes; break; fi
 if grep -q 'source_update_failed' "$native/run/refresh-check.log"; then break; fi
done
cat "$native/run/refresh-check.log"
"$native/control.sh" status
[ "$refreshed" = yes ] || { echo 'PDF extraction passed, but live refresh still needs verification.'; exit 1; }
echo 'PDF extraction and live menu refresh both passed.'
