#!/bin/sh
# Recovery after the confirmed ASUSTOR kernel network-namespace failure.
# Uses host networking with a strictly loopback-bound origin. Filesystem isolation remains.
set -eu
[ "$(id -u)" = 0 ] || { echo 'Run with sudo on the NAS.'; exit 1; }
base=/volume1/dorm-menu
busybox=/volume1/home/admin/dorm-menu-deploy/busybox-probe
report="$base/status/host-services-report.txt"
[ -f "$base/.managed-by-dorm-menu" ] || { echo 'Missing deployment ownership marker'; exit 1; }
umask 077
: > "$report"
chown admin:administrators "$report"
chmod 600 "$report"
exec 3>&1
exec > "$report" 2>&1
docker_timed() { "$busybox" timeout -s KILL 20 docker "$@"; }
fail() { printf 'ERROR: %s\n' "$*"; printf 'Setup stopped. Report: %s\n' "$report" >&3; exit 1; }
for name in dorm-menu-api-host dorm-menu-tunnel-host; do
    if docker_timed inspect "$name" >/dev/null 2>&1; then fail "$name already exists; inspect it before retrying."; fi
done
if /usr/builtin/bin/curl -fsS --connect-timeout 2 --max-time 3 http://127.0.0.1:3000/health >/dev/null 2>&1; then
    fail 'Port 3000 already has a service; no existing service was stopped.'
fi
api_image=$(docker_timed inspect -f '{{.Config.Image}}' dorm-menu-api)
case "$api_image" in dorm-menu-api:armv7-????????????) ;; *) fail 'Unexpected API image.';; esac
tunnel_image=$(sed -n 's/^TUNNEL_IMAGE=//p' /volume1/home/admin/dorm-menu-deploy/images.env)
case "$tunnel_image" in dorm-menu-tunnel:armv7-????????????) ;; *) fail 'Unexpected Tunnel image.';; esac
case "$api_image$tunnel_image" in *[!a-z0-9:.-]*) fail 'Invalid image reference.';; esac
# Retire the failed, never-started bridge container so a Docker restart cannot retry it.
docker_timed update --restart=no dorm-menu-api >/dev/null || fail 'Could not disable retries on failed bridge container.'
grep -q 'hostname: koliapp.lecso.me' "$base/tunnel-config/config.yml" || fail 'Unexpected tunnel hostname.'
sed 's#http://dorm-menu-api:3000#http://127.0.0.1:3000#' "$base/tunnel-config/config.yml" > "$base/tunnel-config/config.new"
chown 10002:10002 "$base/tunnel-config/config.new"
chmod 600 "$base/tunnel-config/config.new"
mv "$base/tunnel-config/config.new" "$base/tunnel-config/config.yml"
grep -q 'service: http://127.0.0.1:3000' "$base/tunnel-config/config.yml" || fail 'Tunnel origin is not loopback.'
set --
if [ -d /sys/fs/cgroup/pids ]; then set -- "$@" --pids-limit 32; fi
if [ -f /sys/fs/cgroup/cpu/cpu.cfs_quota_us ] || [ -f /sys/fs/cgroup/cpu,cpuacct/cpu.cfs_quota_us ]; then set -- "$@" --cpus 0.5; fi
printf 'Starting the loopback-only API...\n' >&3
docker_timed create --name dorm-menu-api-host --label app=dorm-menu --restart unless-stopped \
    --network host --user 10001:10001 --read-only --cap-drop ALL --security-opt no-new-privileges:true \
    --memory 128m "$@" --tmpfs /tmp:rw,noexec,nosuid,nodev,size=8m \
    -e BIND=127.0.0.1:3000 -v "$base/data:/data:rw" -v "$base/api-config:/config:ro" \
    --log-driver json-file --log-opt max-size=2m --log-opt max-file=2 "$api_image" >/dev/null || fail 'API container creation failed.'
docker_timed start dorm-menu-api-host >/dev/null || fail 'API container startup failed.'
ready=false
attempt=0
while [ "$attempt" -lt 20 ]; do
    if /usr/builtin/bin/curl -fsS --connect-timeout 1 --max-time 2 http://127.0.0.1:3000/health > "$base/status/health.json"; then ready=true; break; fi
    attempt=$((attempt+1)); sleep 1
done
if [ "$ready" != true ]; then
    docker_timed logs --tail 40 dorm-menu-api-host || true
    fail 'API health check failed; Tunnel was not started.'
fi
printf 'API liveness passed. Starting Cloudflare Tunnel...\n' >&3
docker_timed create --name dorm-menu-tunnel-host --label app=dorm-menu --restart unless-stopped \
    --network host --user 10002:10002 --read-only --cap-drop ALL --security-opt no-new-privileges:true \
    --memory 128m "$@" --tmpfs /tmp:rw,noexec,nosuid,nodev,size=8m \
    -v "$base/tunnel-config:/config:ro" \
    --log-driver json-file --log-opt max-size=2m --log-opt max-file=2 "$tunnel_image" >/dev/null || fail 'Tunnel container creation failed.'
docker_timed start dorm-menu-tunnel-host >/dev/null || fail 'Tunnel container startup failed.'
printf 'Services started. Waiting ten seconds for source update and Tunnel registration...\n' >&3
sleep 10
printf 'Dorm Menu host-network deployment report\n'
docker_timed inspect -f '{{.Name}} running={{.State.Running}} user={{.Config.User}} readonly={{.HostConfig.ReadonlyRootfs}} network={{.HostConfig.NetworkMode}} restart={{.HostConfig.RestartPolicy.Name}}' dorm-menu-api-host dorm-menu-tunnel-host
printf '\nAPI bind environment (must be loopback):\n'
docker_timed inspect -f '{{range .Config.Env}}{{println .}}{{end}}' dorm-menu-api-host | grep '^BIND='
printf '\nLocal health:\n'; cat "$base/status/health.json"; printf '\n'
printf '\nLocal unauthenticated menu status (expected 401):\n'
/usr/builtin/bin/curl -s --connect-timeout 2 --max-time 3 -o /dev/null -w '%{http_code}\n' http://127.0.0.1:3000/api/v1/menus
printf '\nResource snapshot:\n'
docker_timed stats --no-stream --format '{{.Name}} {{.MemUsage}} {{.CPUPerc}}' dorm-menu-api-host dorm-menu-tunnel-host || true
printf '\nAPI logs:\n'; docker_timed logs --tail 30 dorm-menu-api-host 2>&1 || true
printf '\nTunnel logs:\n'; docker_timed logs --tail 35 dorm-menu-tunnel-host 2>&1 || true
chown -R admin:administrators "$base/status"
chmod 600 "$base/status/"*
printf 'Setup complete. Report: %s\nNo public DNS route has been created yet.\n' "$report" >&3
