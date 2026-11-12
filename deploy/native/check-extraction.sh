#!/bin/sh
set -eu
umask 077
[ "$(id -u)" = 0 ] || exit 1
base=/volume1/dorm-menu
native=$base/native
stage=/volume1/home/admin/dorm-menu-deploy
[ -f "$base/.managed-by-dorm-menu" ] || exit 1
report=$base/status/extraction-report.txt
: > "$report"
chown 999:999 "$report"
chmod 600 "$report"
exec 3>&1
exec >> "$report" 2>&1
trap 'result=$?; tail -n 15 "$report" >&3; exit "$result"' EXIT
# Atomic replacement; running services keep their current executable mapping.
cp "$stage/dorm-launcher-pdfcheck" "$native/bin/dorm-launcher.new"
chown 0:0 "$native/bin/dorm-launcher.new"
chmod 700 "$native/bin/dorm-launcher.new"
mv "$native/bin/dorm-launcher.new" "$native/bin/dorm-launcher"
cp "$stage/diagnostic.pdf" "$native/api-root/data/diagnostic.pdf"
chown 10001:10001 "$native/api-root/data/diagnostic.pdf"
chmod 600 "$native/api-root/data/diagnostic.pdf"
# Preserve stderr so extraction failures are visible rather than discarded.
set +e
"$native/bin/dorm-launcher" api --pdf-check > "$native/run/diagnostic.bbox.html" 2> "$native/run/diagnostic.stderr"
result=$?
set -e
echo "Extraction exit status: $result"
cat "$native/run/diagnostic.stderr"
wc -c "$native/run/diagnostic.bbox.html"
rm -f "$native/api-root/data/diagnostic.pdf"
if [ "$result" = 0 ]; then
 pid=$(cat "$native/run/api.child.pid")
 case "$pid" in *[!0-9]*|'') exit 1;; esac
 [ "$(readlink /proc/"$pid"/exe)" = "$native/api-root/app/dorm-api" ] || exit 1
 kill -USR1 "$pid"
 echo 'PDF check passed; requested an API refresh.'
fi
