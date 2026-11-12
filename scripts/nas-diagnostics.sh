#!/bin/sh
# Read-only compatibility checks. No credentials, Docker environment or full config dump.
set -u
printf 'CPU architecture: '; uname -m
printf 'Kernel: '; uname -r
printf '\nMemory:\n'; free -m
printf '\nDocker client/server:\n'; docker version --format '{{.Client.Version}} / {{.Server.Version}}' 2>/dev/null || docker --version
printf '\nDocker Compose:\n'; docker-compose --version 2>/dev/null || docker compose version 2>/dev/null || true
printf '\nRelevant CPU features:\n'; sed -n '/^Features/p' /proc/cpuinfo | head -n 1
printf '\nAvailable extractors:\n'; command -v pdftotext || true
printf '\nTunnel binary:\n'; command -v cloudflared >/dev/null 2>&1 && cloudflared --version || true
printf '\nSource HTTPS HEAD (no TLS bypass):\n'
curl -I --connect-timeout 10 --max-time 25 'https://bajaionkormanyzat.hu/index.php/kozerdeku/szocialis-intezmenyek/intezmenyi-es-szunidei-gyermeketkeztetes' 2>/dev/null | head -n 1
