"""
Simple standalone test to verify Firecrawl web search functionality.

This test:
1. Starts the universal-search-service with auto-bootstrap
2. Waits for Firecrawl to become available (up to 10 minutes)
3. Tests a simple web search query
4. Reports detailed results

Run with: python test_firecrawl_simple.py
"""

import json
import os
import shutil
import signal
import socket
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from urllib.request import urlopen, Request
from urllib.error import URLError

# Configuration
SERVICE_PORT = 3005
SERVICE_HOST = "127.0.0.1"
BASE_URL = f"http://{SERVICE_HOST}:{SERVICE_PORT}"
FIRECRAWL_PORT = 3002
STARTUP_TIMEOUT = 600  # 10 minutes for Firecrawl to clone + install + start
HEALTH_CHECK_INTERVAL = 5


def http_get(url, timeout=5):
    """Simple HTTP GET using urllib."""
    try:
        req = Request(url, method="GET")
        with urlopen(req, timeout=timeout) as resp:
            body = resp.read().decode("utf-8")
            return resp.status, body
    except URLError as e:
        return None, str(e)
    except Exception as e:
        return None, str(e)


def http_post(url, data, timeout=60):
    """Simple HTTP POST using urllib."""
    try:
        json_data = json.dumps(data).encode("utf-8")
        req = Request(url, data=json_data, method="POST")
        req.add_header("Content-Type", "application/json")
        with urlopen(req, timeout=timeout) as resp:
            body = resp.read().decode("utf-8")
            return resp.status, body
    except URLError as e:
        return None, str(e)
    except Exception as e:
        return None, str(e)


def is_port_open(host: str, port: int) -> bool:
    """Check if a TCP port is open."""
    try:
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            s.settimeout(2)
            s.connect((host, port))
            return True
    except (OSError, socket.timeout):
        return False


def find_binary() -> Path:
    """Find the universal-search-service binary."""
    workspace_root = Path(__file__).parent.parent
    candidates = [
        workspace_root / "target" / "debug" / "universal-search-service",
        workspace_root / "target" / "debug" / "universal-search-service.exe",
        workspace_root / "target" / "release" / "universal-search-service",
        workspace_root / "target" / "release" / "universal-search-service.exe",
    ]
    for c in candidates:
        if c.is_file():
            return c
    return None


def create_config(tmp_dir: Path) -> Path:
    """Create a test configuration file."""
    config = {
        "service": {
            "port": SERVICE_PORT,
            "bind_address": SERVICE_HOST,
        },
        "local_search": {
            "enabled": True,
            "port": 3004,
            "watch_enabled": False,
            "indexes_dir": str(tmp_dir / "indexes"),
            "indexes": [],
        },
        "web_search": {
            "firecrawl": {
                "api_url": "http://localhost:3002",
                "source": "local",
                "auto_start": True,
            },
            "sourcegraph": {
                "access_token": None,
            },
        },
        "bootstrap": {
            "auto_clone_firecrawl": True,
            "auto_install_deps": True,
        },
    }
    config_path = tmp_dir / "config.jsonc"
    config_path.write_text(json.dumps(config, indent=2))
    return config_path


def wait_for_port(port: int, timeout: int, host: str = "127.0.0.1") -> bool:
    """Wait for a port to become open."""
    start = time.time()
    while time.time() - start < timeout:
        if is_port_open(host, port):
            return True
        time.sleep(HEALTH_CHECK_INTERVAL)
    return False


def test_web_search():
    """Test the web search functionality."""
    print()
    print("  Test 1: Health check on service...")
    status, body = http_get(f"{BASE_URL}/health", timeout=5)
    if status == 200:
        print(f"    Service health: OK ({body})")
    else:
        print(f"    Service health: FAIL (status={status}, body={body})")

    print()
    print("  Test 2: Firecrawl health check...")
    status, body = http_get(f"http://localhost:{FIRECRAWL_PORT}/health", timeout=5)
    if status == 200:
        print(f"    Firecrawl health: OK")
    else:
        print(f"    Firecrawl health: FAIL (status={status}, body={body})")

    print()
    print("  Test 3: Web search query...")
    payload = {"query": "rust programming language", "count": 3}
    status, body = http_post(f"{BASE_URL}/web/search", payload, timeout=120)
    if status == 200:
        data = json.loads(body)
        print(f"    Web search: OK")
        print(f"    Query: {data.get('query')}")
        print(f"    Mode: {data.get('mode')}")
        print(f"    Results: {data.get('result_count')}")
        if data.get("results"):
            for i, r in enumerate(data["results"][:2]):
                title = r.get("title", "(no title)")
                url = r.get("url", "(no url)")
                print(f"      [{i + 1}] {title}")
                print(f"          {url}")
    else:
        print(f"    Web search: FAIL (status={status})")
        print(f"    Response: {body[:300] if body else '(empty)'}")

    print()


def main():
    print("=" * 70)
    print("Firecrawl Simple Test")
    print("=" * 70)
    print()

    # Step 0: Check prerequisites
    print("Step 0: Checking prerequisites...")
    binary = find_binary()
    if not binary:
        print("  ERROR: universal-search-service binary not found")
        print("  Run: cargo build --package universal-search-service")
        sys.exit(1)
    print(f"  Binary: {binary}")

    if not is_port_open("localhost", 5432):
        print("  ERROR: PostgreSQL not running on port 5432")
        sys.exit(1)
    print("  PostgreSQL: OK (port 5432)")

    if not is_port_open("localhost", 6379):
        print("  ERROR: Redis not running on port 6379")
        sys.exit(1)
    print("  Redis: OK (port 6379)")

    if not shutil.which("pnpm"):
        print("  ERROR: pnpm not installed")
        sys.exit(1)
    print("  pnpm: OK")

    print()

    # Check if Firecrawl is already running
    if is_port_open("localhost", FIRECRAWL_PORT):
        print("Firecrawl is already running on port 3002!")
        print("Skipping service startup, testing directly...")
        test_web_search()
        return

    # Step 1: Start the service
    print("Step 1: Starting universal-search-service...")
    tmp_dir = Path(tempfile.mkdtemp(prefix="fc_test_"))
    config_path = create_config(tmp_dir)

    env = os.environ.copy()
    env["UNIVERSAL_SEARCH_DIR"] = str(tmp_dir)
    env["UNIVERSAL_SEARCH_CONFIG"] = str(config_path)
    env["RUST_LOG"] = "info"

    proc = subprocess.Popen(
        [str(binary), "--log-level", "info", "run"],
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )

    try:
        # Step 2: Wait for the main service to start
        print("Step 2: Waiting for service to start...")
        if not wait_for_port(SERVICE_PORT, 60):
            print("  ERROR: Service did not start on port 3005")
            proc.kill()
            sys.exit(1)
        print("  Service is running on port 3005")
        print()

        # Step 3: Wait for Firecrawl to start
        print("Step 3: Waiting for Firecrawl to start on port 3002...")
        print("  (This may take 5-10 minutes for clone + install + compile)")
        if not wait_for_port(FIRECRAWL_PORT, STARTUP_TIMEOUT):
            print("  ERROR: Firecrawl did not start on port 3002")
            print("  Attempting test anyway...")
            test_web_search()
            return

        print("  Firecrawl is running on port 3002!")
        print()

        # Step 4: Test web search
        test_web_search()

    finally:
        # Cleanup
        print()
        print("Cleaning up...")
        proc.terminate()
        try:
            proc.wait(timeout=30)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait()
        shutil.rmtree(tmp_dir, ignore_errors=True)
        print("Done.")


if __name__ == "__main__":
    main()
