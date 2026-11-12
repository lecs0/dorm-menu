#!/bin/sh
set -eu
umask 077
[ "$(id -u)" = 0 ] || { echo 'Run with sudo.'; exit 1; }
base=/volume1/dorm-menu
native=$base/native
stage=/volume1/home/admin/dorm-menu-deploy
[ -f "$base/.managed-by-dorm-menu" ] && [ -x "$native/bin/dorm-supervisor" ] || exit 1
cd "$stage"
sha256sum -c WEBSITE-SHA256SUMS
report=$base/status/download-site-report.txt
: > "$report"; chown 999:999 "$report"; chmod 600 "$report"
exec 3>&1
exec >> "$report" 2>&1
progress() { echo "$*"; echo "$*" >&3; }
api=$native/api-root/app/dorm-api
cfg=$native/tunnel-root/config/config.yml
oldapi=$native/api-root/app/dorm-api.before-download-site
oldcfg=$native/tunnel-root/config/config.before-download-site.yml
[ ! -e "$oldapi" ] && [ ! -e "$oldcfg" ] || { progress 'A previous download-site backup exists; stopping without overwriting it.'; exit 1; }
[ "$(grep -c '^  - service: http_status:404$' "$cfg")" = 1 ] || { progress 'Unexpected tunnel configuration; no changes made.'; exit 1; }
grep -q 'service: http://127.0.0.1:3000' "$cfg" || { progress 'Tunnel origin is not the expected local API.'; exit 1; }
grep -q 'hostname: koliapp.lecso.me' "$cfg" || { progress 'Expected hostname missing.'; exit 1; }
cp -p "$api" "$oldapi"; cp -p "$cfg" "$oldcfg"
changed_api=no; changed_tunnel=no
start_api() { "$native/bin/busybox" setsid "$native/bin/dorm-supervisor" run api </dev/null >/dev/null 2>&1 & }
start_tunnel() { "$native/bin/busybox" setsid "$native/bin/dorm-supervisor" run tunnel </dev/null >/dev/null 2>&1 & }
rollback() {
 r=$?
 if [ "$r" -ne 0 ]; then
  echo "Update failed (exit $r); restoring the previous services." >&3
  if [ "$changed_tunnel" = yes ]; then "$native/bin/dorm-supervisor" stop tunnel || true; cp -p "$oldcfg" "$cfg"; start_tunnel; fi
  if [ "$changed_api" = yes ]; then "$native/bin/dorm-supervisor" stop api || true; cp -p "$oldapi" "$api"; start_api; fi
  tail -n 15 "$report" >&3
 fi
}
trap rollback EXIT
awk '/^  - service: http_status:404$/ { print "  - hostname: koliapp.lecso.me"; print "    path: ^/($|Kollegiumi-menu[.]apk$)"; print "    service: http://127.0.0.1:3000" } { print }' "$cfg" > "$cfg.new"
chown 0:10002 "$cfg.new"; chmod 440 "$cfg.new"
cp "$stage/dorm-api-website" "$api.new"; chown 0:0 "$api.new"; chmod 755 "$api.new"
progress 'Updating the API with the download page...'
"$native/bin/dorm-supervisor" stop api
mv "$api.new" "$api"; changed_api=yes
start_api
healthy=no
for n in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15; do
 if /usr/builtin/bin/curl --max-time 2 --silent --fail http://127.0.0.1:3000/health >/dev/null; then healthy=yes; break; fi
 sleep 2
done
[ "$healthy" = yes ] || { echo 'API health check failed.'; exit 1; }
/usr/builtin/bin/curl --max-time 5 --silent --fail http://127.0.0.1:3000/ > "$native/run/website-check.html"
grep -q 'Kollegiumi-menu.apk' "$native/run/website-check.html"
/usr/builtin/bin/curl --max-time 15 --silent --fail http://127.0.0.1:3000/Kollegiumi-menu.apk > "$native/run/website-check.apk"
expected=$(cat "$stage/WEBSITE-APK-SHA256")
actual=$(sha256sum "$native/run/website-check.apk" | cut -d' ' -f1)
[ "$actual" = "$expected" ] || { echo 'Downloaded APK checksum mismatch.'; exit 1; }
rm -f "$native/run/website-check.apk" "$native/run/website-check.html"
progress 'Enabling only the page and APK paths on the existing tunnel...'
"$native/bin/dorm-supervisor" stop tunnel
mv "$cfg.new" "$cfg"; changed_tunnel=yes
before=$(wc -l < "$native/logs/tunnel.log")
start_tunnel
connected=no
for n in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15; do
 sleep 2
 if sed -n "$((before + 1)),\$p" "$native/logs/tunnel.log" | grep -q 'Registered tunnel connection'; then connected=yes; break; fi
done
[ "$connected" = yes ] || { echo 'Tunnel did not reconnect in time.'; exit 1; }
progress 'Download page and APK are installed; API is healthy and tunnel reconnected.'
