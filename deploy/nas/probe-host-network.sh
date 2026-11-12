#!/bin/sh
# Avoid the confirmed broken network-namespace creation path; no service is started.
set -eu
[ "$(id -u)" = 0 ] || { echo 'Run with sudo on the NAS.'; exit 1; }
busybox=/volume1/home/admin/dorm-menu-deploy/busybox-probe
report=/volume1/dorm-menu/status/host-network-probe.txt
umask 077
: > "$report"
chown admin:administrators "$report"
chmod 600 "$report"
exec 3>&1
exec > "$report" 2>&1
docker_timed() { "$busybox" timeout -s KILL 15 docker "$@"; }
printf 'Testing one disposable container; each Docker operation is capped at 15 seconds.\n' >&3
image=$(docker_timed inspect -f '{{.Config.Image}}' dorm-menu-api)
id=$(docker_timed create --label app=dorm-menu-diagnostic --network host \
    --user 10001:10001 --cap-drop ALL --read-only --security-opt no-new-privileges:true \
    --entrypoint /bin/busybox "$image" echo 'Host-network probe OK')
printf 'Probe container: %s\n' "$id"
set +e
docker_timed start -a "$id"
result=$?
set -e
printf 'Docker start status: %s\n' "$result"
if [ "$result" = 0 ]; then
    docker_timed inspect -f 'exit={{.State.ExitCode}} error={{.State.Error}}' "$id"
    docker_timed rm "$id" >/dev/null
    printf 'Probe succeeded; disposable container removed. No ports or folders were exposed.\n' >&3
else
    printf 'Probe failed or timed out; no further probes will run.\n' >&3
fi
printf 'Report: %s\n' "$report" >&3
exit "$result"
