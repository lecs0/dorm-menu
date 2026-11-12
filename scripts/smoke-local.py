#!/usr/bin/env python3
"""Exercise a built Linux API with ephemeral credentials; no token is printed."""
import hashlib
import json
import os
from pathlib import Path
import secrets
import shutil
import socket
import subprocess
import tempfile
import time
import urllib.request
import urllib.error

root = Path(__file__).resolve().parent.parent
binary = root / "target/release/dorm-api"
assert binary.exists(), "Build the release API first"
assert (root / "data/menus.json").exists(), "Run dorm-api check first"
with tempfile.TemporaryDirectory(prefix="dorm-smoke-") as tmp:
    directory = Path(tmp)
    shutil.copy(root / "data/menus.json", directory / "menus.json")
    token = secrets.token_hex(32)
    token_file = directory / "tokens.json"
    token_file.write_text(json.dumps({"smoke": hashlib.sha256(token.encode()).hexdigest()}))
    token_file.chmod(0o600)
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        port = s.getsockname()[1]
    env = dict(os.environ, DATA_DIR=tmp, TOKEN_FILE=str(token_file), BIND=f"127.0.0.1:{port}")
    with (directory / "server.log").open("w") as log:
        process = subprocess.Popen([str(binary)], env=env, stdout=log, stderr=log)
        try:
            base = f"http://127.0.0.1:{port}"
            for attempt in range(100):
                try:
                    with urllib.request.urlopen(base + "/health", timeout=1) as response:
                        assert json.load(response) == {"status": "ok"}
                    break
                except (OSError, urllib.error.URLError):
                    assert process.poll() is None, "API exited before becoming ready"
                    time.sleep(.05)
            else:
                raise AssertionError("API did not start")
            def get(path, authorized=True, etag=None):
                headers = {"Authorization": "Bearer " + token} if authorized else {}
                if etag:
                    headers["If-None-Match"] = etag
                request = urllib.request.Request(base + path, headers=headers)
                try:
                    return urllib.request.urlopen(request, timeout=3)
                except urllib.error.HTTPError as error:
                    return error
            assert get("/api/v1/menus", False).code == 401
            with get("/api/v1/menus") as response:
                etag = response.headers["ETag"]
                snapshot = json.load(response)
                assert snapshot["schema_version"] == 1
                assert len(snapshot["menus"]) >= 1
                assert all("Kollegium" in m["entry"]["source_url"] for m in snapshot["menus"])
            assert get("/api/v1/menus", etag=etag).code == 304
            token_file.write_text("{}")
            assert get("/api/v1/menus").code == 401
            print("PASS: real TCP health, authentication, stored menus, conditional request, token revocation")
        finally:
            process.terminate()
            process.wait(timeout=5)
