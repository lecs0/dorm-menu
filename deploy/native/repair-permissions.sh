#!/bin/sh
set -eu
umask 077
[ "$(id -u)" = 0 ] || { echo 'Run with sudo.'; exit 1; }
base=/volume1/dorm-menu
native=$base/native
[ -f "$base/.managed-by-dorm-menu" ] && [ -x "$native/bin/dorm-launcher" ] || exit 1
report=$base/status/native-permissions-report.txt
: > "$report"
chown 999:999 "$report"
chmod 600 "$report"
exec 3>&1
exec >> "$report" 2>&1
progress() { echo "$*"; echo "$*" >&3; }
trap 'result=$?; if [ "$result" -ne 0 ]; then echo "Repair stopped (exit $result). Last report lines:" >&3; tail -n 15 "$report" >&3; fi' EXIT
"$native/control.sh" stop
for service in api tunnel; do
 jail=$native/$service-root
 chmod 755 "$jail/dev" "$jail/etc"
 chmod 644 "$jail/etc/resolv.conf" "$jail/etc/passwd" "$jail/etc/group"
 [ -c "$jail/dev/random" ] || mknod "$jail/dev/random" c 1 8
 chmod 666 "$jail/dev/null" "$jail/dev/urandom" "$jail/dev/random"
 ls -ld "$jail" "$jail/dev" "$jail/etc"
 ls -l "$jail/dev/urandom" "$jail/etc/resolv.conf"
done
chmod 755 "$native/tunnel-root/bin/cloudflared"
progress 'Checking the API inside its restricted filesystem...'
"$native/bin/dorm-launcher" api --check
"$native/bin/dorm-launcher" tunnel --check
progress 'Starting native services...'
"$native/control.sh" start
healthy=no
for attempt in 1 2 3 4 5 6 7 8 9 10; do
 if /usr/builtin/bin/curl --max-time 2 --silent --fail http://127.0.0.1:3000/health; then healthy=yes; break; fi
 sleep 2
done
if [ "$healthy" != yes ]; then
 tail -n 40 "$native/logs/api.log" || true
 tail -n 20 "$native/logs/tunnel.log" || true
 "$native/control.sh" stop
 progress 'Health check failed; services stopped. Report: native-permissions-report.txt'; exit 1
fi
"$native/control.sh" status
for hook in /usr/local/etc/init.d/S98dorm-menu-native /usr/local/etc/init.d/K98dorm-menu-native; do
 if [ -L "$hook" ] && [ "$(readlink "$hook")" = "$native/control.sh" ]; then :
 elif [ -e "$hook" ] || [ -L "$hook" ]; then progress "Existing unrelated startup hook: $hook"; exit 1
 else ln -s "$native/control.sh" "$hook"
 fi
done
sleep 3
tail -n 30 "$native/logs/api.log"
tail -n 30 "$native/logs/tunnel.log"
progress 'API health check passed. Startup hooks installed; tunnel registration needs verification.'
