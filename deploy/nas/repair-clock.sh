#!/bin/sh
# Replace only the Dorm Menu API with the tested legacy-clock build. Keep seccomp.
set -eu
[ "$(id -u)" = 0 ] || { echo 'Run with sudo on the NAS.'; exit 1; }
cd /volume1/home/admin/dorm-menu-deploy
base=/volume1/dorm-menu
busybox="$PWD/busybox-probe"
report="$base/status/clock-repair-report.txt"
[ -f "$base/.managed-by-dorm-menu" ] || { echo 'Missing ownership marker'; exit 1; }
umask 077
: > "$report"; chown admin:administrators "$report"; chmod 600 "$report"
exec 3>&1
exec > "$report" 2>&1
docker_timed() { "$busybox" timeout -s KILL 20 docker "$@"; }
health() { "$busybox" timeout -s KILL 4 "$busybox" wget -q -O - http://127.0.0.1:3000/health; }
fail() { printf 'ERROR: %s\n' "$*"; printf 'Setup stopped. Report: %s\n' "$report" >&3; exit 1; }
sha256sum -c CLOCK-SHA256SUMS || fail 'Clock update checksum mismatch.'
owner=$(docker_timed inspect -f '{{index .Config.Labels "app"}}' dorm-menu-api-host)
[ "$owner" = dorm-menu ] || fail 'The target API container does not belong to this application.'
if docker_timed inspect dorm-menu-api-host-clock-old >/dev/null 2>&1; then fail 'A previous clock-update backup exists; inspect before retrying.'; fi
api_image=$(sed -n 's/^API_IMAGE=//p' clock-images.env)
tunnel_image=$(sed -n 's/^TUNNEL_IMAGE=//p' clock-images.env)
case "$api_image" in dorm-menu-api:armv7-????????????) ;; *) fail 'Invalid API image.';; esac
case "$tunnel_image" in dorm-menu-tunnel:armv7-????????????) ;; *) fail 'Invalid Tunnel image.';; esac
case "$api_image$tunnel_image" in *[!a-z0-9:.-]*) fail 'Invalid image characters.';; esac
printf 'Stopping the API crash loop and loading the tested clock-compatible image...\n' >&3
docker_timed update --restart=no dorm-menu-api-host >/dev/null
docker_timed stop -t 2 dorm-menu-api-host >/dev/null
docker_timed load -i clock-api-image.tar.gz
printf 'Checking clocks and timed waits under the original Docker seccomp policy...\n' >&3
docker_timed run --rm --network host --user 10001:10001 --cap-drop ALL --read-only \
    --security-opt no-new-privileges:true --memory 128m "$api_image" clock-check || fail 'Clock test failed; no replacement service was started.'
docker_timed rename dorm-menu-api-host dorm-menu-api-host-clock-old
set --
if [ -d /sys/fs/cgroup/pids ]; then set -- "$@" --pids-limit 32; fi
if [ -f /sys/fs/cgroup/cpu/cpu.cfs_quota_us ] || [ -f /sys/fs/cgroup/cpu,cpuacct/cpu.cfs_quota_us ]; then set -- "$@" --cpus 0.5; fi
printf 'Starting the loopback API with all existing security restrictions...\n' >&3
docker_timed create --name dorm-menu-api-host --label app=dorm-menu --restart unless-stopped \
    --network host --user 10001:10001 --read-only --cap-drop ALL --security-opt no-new-privileges:true \
    --memory 128m "$@" --tmpfs /tmp:rw,noexec,nosuid,nodev,size=8m \
    -e BIND=127.0.0.1:3000 -v "$base/data:/data:rw" -v "$base/api-config:/config:ro" \
    --log-driver json-file --log-opt max-size=2m --log-opt max-file=2 "$api_image" >/dev/null
docker_timed start dorm-menu-api-host >/dev/null
ready=false
attempt=0
while [ "$attempt" -lt 15 ]; do
    if health > "$base/status/health.json"; then ready=true; break; fi
    attempt=$((attempt+1)); sleep 1
done
if [ "$ready" != true ]; then docker_timed logs --tail 30 dorm-menu-api-host; fail 'API health check failed.'; fi
printf 'API health passed. Starting Cloudflare Tunnel...\n' >&3
if ! docker_timed inspect dorm-menu-tunnel-host >/dev/null 2>&1; then
    docker_timed create --name dorm-menu-tunnel-host --label app=dorm-menu --restart unless-stopped \
        --network host --user 10002:10002 --read-only --cap-drop ALL --security-opt no-new-privileges:true \
        --memory 128m "$@" --tmpfs /tmp:rw,noexec,nosuid,nodev,size=8m \
        -v "$base/tunnel-config:/config:ro" \
        --log-driver json-file --log-opt max-size=2m --log-opt max-file=2 "$tunnel_image" >/dev/null
else
    owner=$(docker_timed inspect -f '{{index .Config.Labels "app"}}' dorm-menu-tunnel-host)
    [ "$owner" = dorm-menu ] || fail 'Tunnel container ownership mismatch.'
fi
docker_timed start dorm-menu-tunnel-host >/dev/null
printf 'Waiting ten seconds for source parsing and tunnel registration...\n' >&3
sleep 10
printf '\nDeployment status\n'
docker_timed inspect -f '{{.Name}} running={{.State.Running}} restarting={{.State.Restarting}} user={{.Config.User}} readonly={{.HostConfig.ReadonlyRootfs}} network={{.HostConfig.NetworkMode}} security={{json .HostConfig.SecurityOpt}} restart={{.HostConfig.RestartPolicy.Name}}' dorm-menu-api-host dorm-menu-tunnel-host
printf '\nAPI bind:\n'; docker_timed inspect -f '{{range .Config.Env}}{{println .}}{{end}}' dorm-menu-api-host | grep '^BIND='
printf '\nLocal health:\n'; health; printf '\n'
printf '\nLocal unauthenticated menu status:\n'
/usr/builtin/bin/curl -s --connect-timeout 2 --max-time 4 -o /dev/null -w '%{http_code}\n' http://127.0.0.1:3000/api/v1/menus || true
printf '\nResource snapshot:\n'
docker_timed stats --no-stream --format '{{.Name}} {{.MemUsage}} {{.CPUPerc}}' dorm-menu-api-host dorm-menu-tunnel-host || true
printf '\nAPI logs:\n'; docker_timed logs --tail 25 dorm-menu-api-host 2>&1 || true
printf '\nTunnel logs:\n'; docker_timed logs --tail 35 dorm-menu-tunnel-host 2>&1 || true
chown -R admin:administrators "$base/status"; chmod 600 "$base/status/"*
printf 'Setup finished. Report: %s\nThe public DNS route is still unset.\n' "$report" >&3
