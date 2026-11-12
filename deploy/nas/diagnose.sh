#!/bin/sh
# Bounded, disposable probes only. No shares, credentials or host ports are exposed.
set -u
[ "$(id -u)" = 0 ] || { echo 'Run with sudo on the NAS.'; exit 1; }
report=/volume1/dorm-menu/status/runtime-diagnostics.txt
umask 077
: > "$report"
chown admin:administrators "$report"
chmod 600 "$report"
exec 3>&1
exec > "$report" 2>&1
timeout_bin=/volume1/home/admin/dorm-menu-deploy/busybox-probe
[ -x "$timeout_bin" ] || { echo 'Missing bounded diagnostic helper'; exit 1; }
docker_timed() { "$timeout_bin" timeout -s KILL 15 docker "$@"; }
printf 'Collecting diagnostic information; Docker operations have a 15-second timeout.\n' >&3
printf 'Docker/runtime diagnostics\n'
docker_timed version --format 'Docker={{.Server.Version}}'
docker_timed info --format 'Driver={{.Driver}} security={{.SecurityOptions}} running_containers={{.ContainersRunning}} runtimes={{json .Runtimes}}'
docker_timed inspect -f 'State={{json .State}} SecurityOpt={{json .HostConfig.SecurityOpt}}' dorm-menu-api
/usr/local/AppCentral/docker-ce/bin/docker-runc --version 2>&1 || true
printf '\nMemory\n'; free -m
printf '\nRelevant kernel messages\n'
dmesg | grep -iE 'out of memory|oom-killer|killed process|runc|seccomp|segfault|denied' | tail -n 15 || true
image=$(docker_timed inspect -f '{{.Config.Image}}' dorm-menu-api)
probe() {
    label=$1; shift
    printf '\nPROBE: %s\n' "$label"
    # All probes are non-root, capability-free, read-only, without volumes or ports.
    id=$(docker_timed create --label app=dorm-menu-diagnostic --user 10001:10001 \
        --cap-drop ALL --read-only --entrypoint /bin/busybox "$@" "$image" echo "probe OK: $label") || return
    printf 'Testing: %s\n' "$label" >&3
    docker_timed start -a "$id"
    result=$?
    if [ "$result" -eq 137 ] || [ "$result" -eq 124 ]; then
        printf 'Docker start timed out for probe %s (%s). No more probes will be started.\n' "$id" "$label"
        printf 'Docker start timed out. Report is available at %s\n' "$report" >&3
        exit 1
    fi
    docker_timed inspect -f 'exit={{.State.ExitCode}} error={{.State.Error}}' "$id"
    docker_timed rm "$id" >/dev/null 2>&1 || true
    printf 'probe command status=%s\n' "$result"
}
probe 'strict minimal' --network none --security-opt no-new-privileges:true
probe 'minimal without no-new-privileges flag' --network none
probe 'strict private bridge' --network dorm-menu-private --security-opt no-new-privileges:true
probe 'strict memory limit' --network none --memory 128m --security-opt no-new-privileges:true
probe 'strict CPU quota' --network none --cpus 0.5 --security-opt no-new-privileges:true
probe 'strict temporary mount' --network none --tmpfs /tmp:rw,noexec,nosuid,nodev,size=8m --security-opt no-new-privileges:true
printf '\nDiagnostic probes finished; application containers were not changed.\n'
printf 'Diagnostic complete. Report: %s\n' "$report" >&3
chown admin:administrators "$report"
chmod 600 "$report"
