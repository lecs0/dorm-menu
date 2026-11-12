#!/bin/sh
set -eu
base=/volume1/dorm-menu/native
[ "$(id -u)" = 0 ] || exit 1
case "${1:-}" in
start)
 for service in api tunnel; do
  for controller in memory cpu; do
   group=/sys/fs/cgroup/$controller/dorm-menu-$service-native
   mkdir -p "$group"
   if [ "$controller" = memory ]; then
    echo 134217728 > "$group/memory.limit_in_bytes"
   else
    echo 100000 > "$group/cpu.cfs_period_us"
    echo 50000 > "$group/cpu.cfs_quota_us"
   fi
  done
 done
 "$base/bin/busybox" setsid "$base/bin/dorm-supervisor" run api </dev/null >/dev/null 2>&1 &
 "$base/bin/busybox" setsid "$base/bin/dorm-supervisor" run tunnel </dev/null >/dev/null 2>&1 &
 ;;
stop)
 "$base/bin/dorm-supervisor" stop tunnel
 "$base/bin/dorm-supervisor" stop api
 ;;
status)
 for service in api tunnel; do
  echo "$service:"
  if [ -f "$base/run/$service.child.pid" ]; then
   pid=$(cat "$base/run/$service.child.pid")
   case "$pid" in *[!0-9]*|'') exit 1;; esac
   sed -n '/^Name:/p;/^State:/p;/^Uid:/p;/^VmRSS:/p;/^CapEff:/p;/^CapBnd:/p;/^NoNewPrivs:/p;/^Seccomp:/p' /proc/"$pid"/status
  fi
 done
 /usr/builtin/bin/curl --max-time 5 --fail --silent http://127.0.0.1:3000/health
 ;;
*) echo "Usage: $0 start|stop|status"; exit 2;;
esac
