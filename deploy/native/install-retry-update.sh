#!/bin/sh
set -eu
umask 077
[ "$(id -u)" = 0 ] || { echo 'Run with sudo.'; exit 1; }
base=/volume1/dorm-menu
native=$base/native
stage=/volume1/home/admin/dorm-menu-deploy
[ -f "$base/.managed-by-dorm-menu" ] && [ -x "$native/bin/dorm-supervisor" ] || exit 1
cd "$stage"
sha256sum -c RETRY-SHA256SUMS
report=$base/status/retry-update-report.txt
: > "$report"
chown 999:999 "$report"
chmod 600 "$report"
exec 3>&1
exec >> "$report" 2>&1
trap 'r=$?; if [ "$r" -ne 0 ]; then echo "Update stopped (exit $r). Report:" >&3; tail -n 20 "$report" >&3; fi' EXIT
api=$native/api-root/app/dorm-api
backup=$native/api-root/app/dorm-api.before-retry
[ ! -e "$backup" ] || { echo 'Backup already exists; refusing to overwrite.'; exit 1; }
cp "$api" "$backup"
cp "$stage/dorm-api-retry" "$api.new"
chown 0:0 "$api.new"
chmod 755 "$api.new"
"$native/bin/dorm-supervisor" stop api
mv "$api.new" "$api"
"$native/bin/busybox" setsid "$native/bin/dorm-supervisor" run api </dev/null >/dev/null 2>&1 &
healthy=no
for n in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15; do
 if /usr/builtin/bin/curl --max-time 2 --silent --fail http://127.0.0.1:3000/health >/dev/null; then healthy=yes; break; fi
 sleep 2
done
if [ "$healthy" != yes ]; then
 echo 'New API failed health check; restoring previous API.'
 "$native/bin/dorm-supervisor" stop api || true
 mv "$backup" "$api"
 "$native/bin/busybox" setsid "$native/bin/dorm-supervisor" run api </dev/null >/dev/null 2>&1 &
 exit 1
fi
sleep 5
tail -n 12 "$native/logs/api.log"
echo 'API update installed and healthy.' >&3
echo 'API update installed and healthy.'
