#!/bin/sh
set -eu
umask 077
[ "$(id -u)" = 0 ] || { echo 'Run this installer with sudo.'; exit 1; }
cd /volume1/home/admin/dorm-menu-deploy
sha256sum -c NATIVE-SHA256SUMS
base=/volume1/dorm-menu
[ -f "$base/.managed-by-dorm-menu" ] || { echo 'Existing managed deployment required.'; exit 1; }
[ ! -e "$base/native" ] || { echo 'Native directory already exists; stopping to preserve it.'; exit 1; }
for hook in /usr/local/etc/init.d/S98dorm-menu-native /usr/local/etc/init.d/K98dorm-menu-native; do
 [ ! -e "$hook" ] || { echo "Hook already exists: $hook"; exit 1; }
done
report=$base/status/native-install-report.txt
: > "$report"
chown 999:999 "$report"
chmod 600 "$report"
exec 3>&1
exec >> "$report" 2>&1
progress() { echo "$*"; echo "$*" >&3; }
progress 'Stopping only the previous dorm-menu Docker services...'
for name in dorm-menu-api dorm-menu-api-host dorm-menu-api-host-clock-old dorm-menu-tunnel dorm-menu-tunnel-host; do
 label=$(./busybox timeout 10 docker inspect --format '{{index .Config.Labels "app"}}' "$name" 2>/dev/null || true)
 if [ "$label" = dorm-menu ]; then
  ./busybox timeout 10 docker update --restart=no "$name" || true
  ./busybox timeout 10 docker stop -t 2 "$name" || true
 fi
done
if /usr/builtin/bin/curl --max-time 2 --silent --fail http://127.0.0.1:3000/health >/dev/null; then
 progress 'Port 3000 is still serving an application; stopping without replacing it.'; exit 1
fi
progress 'Installing isolated native service files...'
mkdir "$base/native"
tar -xzf native-bundle.tar.gz -C "$base/native"
native=$base/native
chown -R 0:0 "$native"
chmod 755 "$native/tunnel-root/bin/cloudflared"
chmod 700 "$native" "$native/bin" "$native/logs" "$native/run"
for service in api tunnel; do
 jail=$native/$service-root
 uid=10001; [ "$service" = api ] || uid=10002
 mkdir -p "$jail/dev" "$jail/etc" "$jail/config" "$jail/tmp" "$jail/data"
 cp /etc/resolv.conf "$jail/etc/resolv.conf"
 printf 'dormsvc:x:%s:%s:Dorm Menu:/data:/bin/false\n' "$uid" "$uid" > "$jail/etc/passwd"
 printf 'dormsvc:x:%s:\n' "$uid" > "$jail/etc/group"
 chmod 755 "$jail/dev" "$jail/etc"
 chmod 644 "$jail/etc/resolv.conf" "$jail/etc/passwd" "$jail/etc/group"
 mknod "$jail/dev/null" c 1 3
 mknod "$jail/dev/urandom" c 1 9
 mknod "$jail/dev/random" c 1 8
 chmod 666 "$jail/dev/null" "$jail/dev/urandom" "$jail/dev/random"
 chown "$uid:$uid" "$jail/data" "$jail/tmp"
 chmod 700 "$jail/data" "$jail/tmp"
 chown "0:$uid" "$jail/config"
 chmod 750 "$jail/config"
done
cp -a "$base/data/." "$native/api-root/data/"
chown -R 10001:10001 "$native/api-root/data"
cp "$base/api-config/tokens.json" "$native/api-root/config/tokens.json"
chown 0:10001 "$native/api-root/config/tokens.json"
chmod 440 "$native/api-root/config/tokens.json"
cp "$base/tunnel-config/tunnel.json" "$native/tunnel-root/config/tunnel.json"
cp "$base/tunnel-config/config.yml" "$native/tunnel-root/config/config.yml"
chown 0:10002 "$native/tunnel-root/config/"*
chmod 440 "$native/tunnel-root/config/"*
progress 'Starting native services with separate UIDs, filesystem jails, and resource limits...'
"$native/control.sh" start
healthy=no
for attempt in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15; do
 if /usr/builtin/bin/curl --max-time 2 --silent --fail http://127.0.0.1:3000/health; then healthy=yes; break; fi
 sleep 2
done
if [ "$healthy" != yes ]; then
 tail -n 30 "$native/logs/api.log" || true
 "$native/control.sh" stop
 progress 'Native API did not pass health checks. Services stopped; see native-install-report.txt.'; exit 1
fi
"$native/bin/dorm-launcher" api --check
"$native/bin/dorm-launcher" tunnel --check
"$native/control.sh" status
ln -s "$native/control.sh" /usr/local/etc/init.d/S98dorm-menu-native
ln -s "$native/control.sh" /usr/local/etc/init.d/K98dorm-menu-native
sleep 3
tail -n 25 "$native/logs/tunnel.log" || true
progress 'Native API is healthy. Boot hooks installed. Next: verify the tunnel before creating public DNS.'
