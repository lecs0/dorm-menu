# NAS deployment

## Compatibility status

The NAS was specified as ASUSTOR AS1002T v2, ADM 4.0.7, ARMv7 and 512 MB RAM. SSH diagnostics confirmed ARMv7l, VFPv3/NEON, 498 MB RAM, and kernel 3.10.70. The Docker client is 18.03.1-ce (API 1.37); the admin SSH user cannot access its root-owned daemon socket and passwordless sudo is unavailable. The user supplied `sudo docker version` output confirming the daemon is also 18.03.1-ce/API 1.37 (linux/arm). Container execution remains unverified. Poppler and cloudflared were absent from PATH. The NAS successfully reached the municipality page over verified HTTPS. Do not infer full compatibility from the model name alone. Inspect on the NAS:

```sh
uname -m
uname -r
cat /proc/cpuinfo
free -m
docker version
docker info
```

Avoid sharing complete diagnostic output publicly; it may include local host details. Check ARMv7 **hard-float** support, kernel/seccomp capabilities, Docker API version, available storage and memory. Recent base images, Go runtimes and Rust syscalls can fail on an old kernel even when the CPU architecture matches.

On 2026-11-11 the Docker Hub manifests for [rust:1.99.1-bookworm](https://hub.docker.com/v2/repositories/library/rust/tags/1.99.1-bookworm) and [debian:bookworm-slim](https://hub.docker.com/v2/repositories/library/debian/tags/bookworm-slim) both listed `linux/arm/v7`. This verifies advertised image availability only. Neither Docker image was built here. The shared crate type-checks for `armv7-unknown-linux-gnueabihf`; the full API was also cross-compiled as a static `armv7-unknown-linux-musleabihf` executable using Zig 0.13.0/cargo-zigbuild 0.24.1 and successfully run on this NAS. A private Ubuntu Jammy ARMhf Poppler 22.02.0 bundle extracted both real PDFs during the live NAS refresh. The Debian container image itself remains unbuilt/untested.

## Build on the laptop

Docker buildx and ARM emulation must be configured on the stronger development machine. The following builds within an ARMv7 Debian builder via QEMU, rather than assuming a host-compiled binary will link on ARM:

```sh
docker buildx build --platform linux/arm/v7 -f api/Dockerfile -t dorm-menu:local --load .
docker save dorm-menu:local -o /tmp/dorm-menu-armv7.tar
scp /tmp/dorm-menu-armv7.tar USER@NAS:/YOUR/APP/DIRECTORY/
```

Pin the tested image digest after the first successful deployment. Buildx/QEMU compilation is slower but simpler than matching a cross linker and C library. An alternative native cross build needs `arm-linux-gnueabihf-gcc`, Rust's ARMv7 target and matching libraries, including ring's C/assembly build requirements; don't claim that `cargo check` produces a deployable binary.

On the NAS, create a dedicated application directory, copy `deploy/docker-compose.yml`, and prepare `data/` and `config/`. Generate tokens with `dorm-api token` on the laptop; create `config/tokens.json` as described in the README. Never commit verifiers or real tokens.

```sh
mkdir -p data config
chmod 700 data config
# Write config/tokens.json with one SHA-256 verifier per device, then:
chmod 600 config/tokens.json
# Set ownership appropriate to container uid/gid 10001 (may require NAS admin rights):
sudo chown -R 10001:10001 data config
docker load -i dorm-menu-armv7.tar
docker-compose up -d
docker-compose logs --tail=50 api
curl http://127.0.0.1:3000/health
```

Compose uses the older version 2.4 schema to suit older installations, but each feature still needs checking against your Docker/Compose versions. The API is non-root, with a read-only root filesystem, only app data writable, dropped capabilities, bounded pids/memory/CPU and rotated logs. Limits are starting values, not measured NAS budgets. Keep the extractor's memory headroom in mind; if OOM occurs, the last persisted menu survives. Do not disable kernel protections blindly if the old engine rejects a setting. Identify the exact unsupported feature before adapting the configuration.

No SQL service, public admin endpoint or host paths for arbitrary API input exist. Avoid running unrelated NAS workloads with the same Unix UID used for app data.

## Cloudflare Tunnel

Cloudflare's official [downloads page](https://developers.cloudflare.com/cloudflare-one/networks/connectors/cloudflare-tunnel/downloads/) offers a Linux ARM binary, separately from ARM64. That alone does not verify ARMv7/ADM kernel compatibility. On the NAS, download a specific supported release from Cloudflare's official release link, verify its published checksum, then test `file cloudflared` and `./cloudflared --version`. Only proceed if it runs. Do not substitute an ARM64 binary. Test outbound connectivity according to Cloudflare's [tunnel firewall documentation](https://developers.cloudflare.com/cloudflare-one/networks/connectors/cloudflare-tunnel/configure-tunnels/tunnel-with-firewall/).

Prefer a tunnel installed on the host with the API port bound to loopback. Use the Cloudflare dashboard to create a tunnel and attach `menu.YOURDOMAIN` to `http://127.0.0.1:3000`, or use the locally managed configuration example in `deploy/cloudflared.example.yml`. For locally managed tunnels, create the tunnel and DNS route using the official [setup documentation](https://developers.cloudflare.com/cloudflare-one/networks/connectors/cloudflare-tunnel/get-started/). Keep tunnel credentials readable only by its service account. The example only publishes menu paths; health remains local. No router port-forward is needed for an outbound tunnel.

If cloudflared runs in a separate container, `127.0.0.1` refers to that container. Put both on a private Docker network and target `http://api:3000`; remove the host port mapping if no host tunnel needs it. Keep their credentials and application data separate. Do not expose ADM's web interface through this tunnel.

If the ARM binary or image fails on the NAS's kernel/engine, run a current cloudflared on another always-on Linux device (for example a small PC/Raspberry Pi). Bind the NAS API to a private LAN address and firewall it so only that device can connect. That LAN hop is HTTP unless you configure and validate origin TLS; Cloudflare's public HTTPS does not encrypt that hop. A private VPN is another option if every selected user can install its client. Moving the API to the other device may be simpler if Poppler or the base image also fails on ADM.

The app sends its own device bearer token over public HTTPS. Cloudflare Tunnel does not replace that authentication. Cloudflare Access is not enabled in this implementation: interactive Access would add a browser login, while service-token Access would require another credential flow and header integration in the app. Don't put a permanent shared Access secret into the APK.

Cloudflare is an external dependency with an account, domain/DNS requirements and plan-dependent features/limits. Check your current [Cloudflare plan](https://www.cloudflare.com/plans/) and Access/Tunnel terms before relying on cost or WAF features. The official cloudflared ARM release **2026.10.7** was downloaded, its SHA-256 matched the GitHub release asset digest (`74f690f6707182839b63e0465726c7776dd63f0a174390b911d5d71f35cab94e`), and its `--version` command ran successfully over SSH on this NAS. This verifies binary startup on ADM's kernel, not tunnel connectivity, idle memory, service installation, or reliability. No tunnel/account credentials were configured. No pricing tier has been assumed.

## Operation

- Force a source check while the service runs: `docker-compose kill -s USR1 api`. This signals the process; it does not stop the container.
- Read bounded logs with `docker-compose logs --tail=100 api`. Logs identify source failures without dumping credentials or HTTP headers.
- Revoke a token by writing a replacement JSON file in `config/` and atomically renaming it over `tokens.json`. Mounting the directory lets the process see replacements immediately. Use distinct device IDs. A malformed verifier file fails authentication closed.
- Back up `data/menus.json` and the restricted verifier configuration. The menu JSON is atomically replaced; snapshot it after a successful update or stop the service for a consistent full backup. Tunnel credentials need a separate restricted backup.
- Restore the files with UID 10001 ownership, then restart. No menu history beyond the small current/upcoming/fallback set is guaranteed.
- For upgrades, build and test a new pinned image on the laptop, back up the JSON, load it, update the image tag and restart. Keep the old image and backup for rollback. Patch Poppler and the base OS regularly.

## Standalone compatibility test

A static ARMv7 build can avoid relying on ADM's C library for the API itself:

```sh
rustup target add armv7-unknown-linux-musleabihf
cargo install cargo-zigbuild --version 0.24.1 --locked
# Install Zig 0.13.0 from its official distribution and put zig on PATH.
./scripts/build-armv7.sh
```

Poppler still needs a compatible runtime. For the temporary NAS test, signed Ubuntu Jammy ARMhf package indexes and `--download-only` were used on the laptop, and `dpkg-deb -x` extracted library packages into a private bundle. No package maintainer scripts or NAS system installation were used. Its wrapper invokes its bundled ARMhf ELF loader with an explicit private library search path. The private bundle successfully ran `pdftotext 22.02.0` on the NAS and the API completed a fresh live download/parse of both menu PDFs. Health, auth, ETag 304 and immediate token revocation passed on NAS loopback. After refresh, API VmRSS was 3,652 kB and CPU ticks remained 10 → 10 over a three-second idle sample. This is a short diagnostic, not a peak-memory, sustained-idle or reliability benchmark. The test service was stopped afterward.

This is a compatibility diagnostic, not the primary production deployment: the package set must be kept patched, copyright/source obligations preserved if distributed, and untrusted PDF processing should run under a dedicated restricted account/container rather than the NAS admin account. Do not replace ADM's system libc or put bundled libraries into global library paths.

The optional `scripts/prepare-nas-bundle.sh` automates the same package-bundle recipe on a Zorin/Ubuntu laptop after the static build. It writes `target/dorm-menu-nas-bundle.tar.gz`, includes package versions and copyright files, and performs download/extraction only. The helper itself has only been syntax-checked; the equivalent manual recipe was tested on this NAS. Extract into a dedicated application directory under a restricted service account, supply `config/tokens.json` with mode 600, and use `run.sh check` followed by `run.sh` for local testing. It creates no ADM startup registration or tunnel. Production isolation/restart setup still needs an operator decision.

## Deployment attempt: Docker runtime failure (2026-11-11)

The legacy-format API and Tunnel images loaded successfully on Docker 18.03.1.
The API container was created with the restricted configuration, but Docker failed
before starting its application process:

```
OCI runtime create failed: container_linux.go:348 ...
process_linux.go:301: running exec setns process for init caused "signal: broken pipe"
```

The installer stopped before creating/starting the Tunnel container. No DNS route
was published. Application data and tunnel credentials remain in the dedicated
`/volume1/dorm-menu` directory; the images remain in Docker. The failed API
container must be inspected before attempting a first-install script again.

A direct unprivileged kernel probe using the supplied ARM BusyBox successfully
set no-new-privileges. That does not prove the old Docker/runc integration can
apply it correctly, and is not sufficient to identify the failure's cause.

`deploy/nas/diagnose.sh` is uploaded alongside the installer. Run it with sudo in
the NAS terminal. It collects Docker/runtime versions, relevant kernel messages,
and uses disposable non-root, capability-free containers without volumes or
published ports to compare the runtime settings. The report is saved as
`/volume1/dorm-menu/status/runtime-diagnostics.txt` for inspection over SSH.
The application configuration and other existing containers are not changed.
No runtime replacement, global Docker restart, or production security-control
removal has been performed.

### Confirmed kernel network-namespace failure

The collected kernel log reports `Internal error: Oops` in the runc child, with
`ops_init -> setup_net -> copy_net_ns -> create_new_namespaces -> SyS_unshare`.
The later diagnostic child is stuck in D state with wait channel `copy_net_ns`.
Docker uses the vfs storage driver, default seccomp profile, and runc 1.0.0-rc5.
The API process had not started. This is evidence of a failure in the NAS kernel's
network-namespace creation path; it does not establish the underlying vendor
module defect or that replacing runc would fix it.

The diagnostic shell and Docker client were stopped. A D-state runtime child
remains; do not assume killing its client resolved the kernel wait. No global
Docker restart, NAS reboot or firmware change has been performed.

`probe-host-network.sh` tests one non-root, capability-free, read-only container
with no volumes, no listener, and host networking, avoiding new network namespaces.
Every Docker operation has a 15-second timeout and progress is visible in the
terminal. Production host networking has not yet been adopted. If it is used,
the API must bind to 127.0.0.1 and the tunnel must target that loopback address.
It preserves filesystem/user/capability restrictions but reduces network isolation.

### ARM musl clock compatibility

The host-network API subsequently reached its Rust runtime but exited with
`Operation not permitted` in `std::sys::pal::unix::time`. A process-local test
filter denying ARM time64 syscall 403 reproduced that exact panic on the NAS.
The opt-in `nas-legacy-clock` feature interposes musl's time64 clock-read aliases
and uses the already supported ARM clock syscall 263, widening the result into
the explicit 16-byte musl time64 ABI. It does not change Docker's seccomp policy
or grant extra privileges. Both public Rust clock reads and internal musl timed
waits use the compatibility implementation. It is restricted to little-endian
ARM/musl builds and inherits the old kernel's signed time32 / January 2038 limit.

The replacement binary passed the NAS `clock-check` command (UTC time, monotonic
clock and a timed condition-variable wait), then menu discovery, with syscall
403 denied. It also passed clock-check with the entire ARM time64 syscall range
403–423 denied. Full container/source-refresh validation is the next deployment
step. The original binary's failure and replacement's success were observed,
not inferred from a container image's architecture label.

`repair-clock.sh` performs a clock check inside the original Docker seccomp
policy before replacing the API container. It preserves configuration/menu data,
retires the crash-looping container, keeps loopback-only host networking and all
other container restrictions, and starts the Tunnel only after local health
passes. Every Docker operation is timed. Health checks use explicit helper
paths: sudo on ADM does not include `/usr/builtin/bin` in PATH.

Native deployment for the ADM 3.10 ARMv7 NAS
------------------------------------------

Use `deploy/native/install-native.sh` for the prepared, existing managed NAS
installation. Docker on this device has demonstrated network namespace kernel
faults and incompatible syscall filtering. The native installation does not
create namespaces or change Docker's global configuration.

The root launcher enters a separate filesystem jail for each service, drops to
UID 10001 (API) or 10002 (tunnel), clears capabilities, enables no-new-privileges,
and blocks selected dangerous syscalls. Only application data and temporary
files are writable by the service; NAS shares are absent from the jail. Each
service has a 128 MiB memory cgroup and half-core CPU quota. These restrictions
share the NAS kernel and network; they are not full container isolation.

Root supervisors restart the services with bounded logs. The installer adds
only its own S98/K98 ADM boot hooks. To inspect or stop the installation:

```sh
sudo /volume1/dorm-menu/native/control.sh status
sudo /volume1/dorm-menu/native/control.sh stop
```

Installation progress is also saved in
`/volume1/dorm-menu/status/native-install-report.txt`. Public DNS is a separate
step after API health and Cloudflare tunnel registration are verified.

When a source refresh fails, the API retries after approximately 5, 10, 20,
40 and then 60 minutes, with up to one minute of jitter and a maximum of the
normal check interval. A successful refresh restores the usual 12/24-hour
schedule. The `source_next_check` log event records the selected delay.

The download page is embedded into the API from `website/dist/index.html` and
`website/dist/Kollegiumi-menu.apk`. The APK currently is the explicitly shared
Android debug build; it contains no API token. The tunnel must route only `/`
and `/Kollegiumi-menu.apk` to the API in addition to the protected menu routes.
A future APK is not automatically published: copy the intended APK to that
path, rebuild the native ARM API, and deploy the new binary. Use a stable,
privately held release signing key for a future production release.
