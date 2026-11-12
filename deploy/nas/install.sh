#!/bin/sh
# Reviewed, one-time root installation for the dedicated Dorm Menu containers only.
set -eu
umask 077
[ "$(id -u)" = 0 ] || { echo 'Run this installer with sudo in your NAS terminal.' >&2; exit 1; }
cd "$(dirname "$0")"
staging=$(pwd)
base=/volume1/dorm-menu
network=dorm-menu-private
label=dorm-menu
fail() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }
command -v docker >/dev/null || fail 'Docker is not in PATH.'
docker info >/dev/null 2>&1 || fail 'Docker daemon is not available.'
sha256sum -c SHA256SUMS || fail 'Deployment checksum mismatch.'
api_image=$(sed -n 's/^API_IMAGE=//p' images.env)
tunnel_image=$(sed -n 's/^TUNNEL_IMAGE=//p' images.env)
case "$api_image" in dorm-menu-api:armv7-????????????) ;; *) fail 'Unexpected API image name.';; esac
case "$tunnel_image" in dorm-menu-tunnel:armv7-????????????) ;; *) fail 'Unexpected tunnel image name.';; esac
case "$api_image$tunnel_image" in *[!a-z0-9:.-]*) fail 'Invalid image reference characters.';; esac
for name in dorm-menu-api dorm-menu-tunnel; do
    if docker inspect "$name" >/dev/null 2>&1; then
        fail "Container $name already exists. Inspect it before using the separate update procedure."
    fi
done
if [ -e "$base" ] && [ ! -f "$base/.managed-by-dorm-menu" ]; then
    fail "$base already exists without our ownership marker; refusing to overwrite it."
fi
if [ -e "$base/data/menus.json" ] || [ -e "$base/api-config/tokens.json" ]; then
    fail 'Existing application data found; this first-install script never overwrites a deployment.'
fi
mkdir -p "$base/data" "$base/api-config" "$base/tunnel-config" "$base/status"
chown root:root "$base"
chmod 711 "$base"
printf 'Dorm Menu deployment\n' > "$base/.managed-by-dorm-menu"
cp api-config/tokens.json "$base/api-config/tokens.json"
cp tunnel-config/config.yml "$base/tunnel-config/config.yml"
cp tunnel-config/tunnel.json "$base/tunnel-config/tunnel.json"
if [ -f seed-menus.json ]; then cp seed-menus.json "$base/data/menus.json"; fi
chown -R 10001:10001 "$base/data" "$base/api-config"
chown -R 10002:10002 "$base/tunnel-config"
chmod 700 "$base/data" "$base/api-config" "$base/tunnel-config"
chmod 600 "$base/api-config/tokens.json" "$base/tunnel-config/config.yml" "$base/tunnel-config/tunnel.json"
chown admin:administrators "$base/status"
chmod 700 "$base/status"
docker load -i api-image.tar.gz
docker load -i tunnel-image.tar.gz
if docker network inspect "$network" >/dev/null 2>&1; then
    owner=$(docker network inspect -f '{{index .Labels "app"}}' "$network")
    [ "$owner" = "$label" ] || fail 'The requested Docker network belongs to another application.'
else
    docker network create --driver bridge --label "app=$label" "$network" >/dev/null
fi
# The NAS kernel has no pids controller. Use a pids limit only on kernels that expose it.
set --
if [ -d /sys/fs/cgroup/pids ]; then
    set -- "$@" --pids-limit 32
else
    echo 'NOTE: this kernel has no pids cgroup; a Docker process-count limit is unavailable.'
fi
if [ -f /sys/fs/cgroup/cpu/cpu.cfs_quota_us ] || [ -f /sys/fs/cgroup/cpu,cpuacct/cpu.cfs_quota_us ]; then
    set -- "$@" --cpus 0.5
else
    echo 'NOTE: CFS CPU quotas are unavailable; using the application extraction timeout.'
fi
# No host ports, Docker socket, NAS shares or management directories are mounted.
docker create --name dorm-menu-api --label "app=$label" --restart unless-stopped \
    --user 10001:10001 --read-only --cap-drop ALL --security-opt no-new-privileges:true \
    --memory 128m "$@" --network "$network" \
    --tmpfs /tmp:rw,noexec,nosuid,nodev,size=8m \
    -v "$base/data:/data:rw" -v "$base/api-config:/config:ro" \
    --log-driver json-file --log-opt max-size=2m --log-opt max-file=2 "$api_image" >/dev/null
docker start dorm-menu-api >/dev/null
ready=false
attempt=0
while [ "$attempt" -lt 30 ]; do
    if docker exec dorm-menu-api /bin/wget -q -O - http://127.0.0.1:3000/health > "$base/status/health.json" 2>/dev/null; then ready=true; break; fi
    attempt=$((attempt+1)); sleep 1
done
if [ "$ready" != true ]; then
    docker logs --tail 40 dorm-menu-api >&2
    fail 'API failed its local health check. Tunnel has not been started.'
fi
docker create --name dorm-menu-tunnel --label "app=$label" --restart unless-stopped \
    --user 10002:10002 --read-only --cap-drop ALL --security-opt no-new-privileges:true \
    --memory 128m "$@" --network "$network" \
    --tmpfs /tmp:rw,noexec,nosuid,nodev,size=8m \
    -v "$base/tunnel-config:/config:ro" \
    --log-driver json-file --log-opt max-size=2m --log-opt max-file=2 "$tunnel_image" >/dev/null
docker start dorm-menu-tunnel >/dev/null
# Wait for source parsing and tunnel registration before collecting this report.
sleep 10
{
    echo 'Dorm Menu first-install report'
    docker version --format 'Docker server: {{.Server.Version}}'
    docker inspect -f '{{.Name}} running={{.State.Running}} user={{.Config.User}} read_only={{.HostConfig.ReadonlyRootfs}} restart={{.HostConfig.RestartPolicy.Name}}' dorm-menu-api dorm-menu-tunnel
    echo 'Host port bindings (expected: none):'
    docker port dorm-menu-api
    docker port dorm-menu-tunnel
    echo 'Local API liveness:'
    cat "$base/status/health.json"
    echo
    echo 'Memory/CPU snapshot:'
    docker stats --no-stream --format '{{.Name}} {{.MemUsage}} {{.CPUPerc}}' dorm-menu-api dorm-menu-tunnel || true
    echo 'Docker security options:'
    docker info --format '{{.SecurityOptions}}'
    echo 'API logs:'
    docker logs --tail 20 dorm-menu-api 2>&1
    echo 'Tunnel logs:'
    docker logs --tail 30 dorm-menu-tunnel 2>&1
} > "$base/status/installation-report.txt"
chown -R admin:administrators "$base/status"
chmod 600 "$base/status/"*
cat "$base/status/installation-report.txt"
printf '\nInstallation finished. No DNS route has been created by this script.\n'
printf 'Report: %s/status/installation-report.txt\n' "$base"
