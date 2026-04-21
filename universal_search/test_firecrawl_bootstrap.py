"""
Firecrawl Bootstrap and Test Script

This script:
1. Checks prerequisites (PostgreSQL, Redis, pnpm, git)
2. Clones Firecrawl repo if not present
3. Installs Firecrawl dependencies
4. Starts Firecrawl server
5. Waits for it to become healthy
6. Runs a test search query
7. Stops the server on completion

All files are kept in the universal_search/ project folder.
"""

import json
import os
import shutil
import signal
import socket
import subprocess
import sys
import time
from pathlib import Path

# Configuration
FC_DIR = Path(__file__).parent / "firecrawl"
FC_API_DIR = FC_DIR / "apps" / "api"
FC_PORT = 3002
CLONE_URL = "https://github.com/firecrawl/firecrawl.git"
BUILD_TIMEOUT = 600  # 10 minutes for clone + install
STARTUP_TIMEOUT = 60  # 1 minute for server to start
TEST_TIMEOUT = 30


def is_port_open(host: str, port: int) -> bool:
    """Check if a TCP port is open."""
    try:
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            s.settimeout(2)
            s.connect((host, port))
            return True
    except (OSError, socket.timeout):
        return False


def check_prerequisites() -> list[str]:
    """Check if all required tools are available. Returns list of missing items."""
    missing = []

    # Check PostgreSQL
    if not is_port_open("localhost", 5432):
        missing.append("PostgreSQL (port 5432)")

    # Check Redis
    if not is_port_open("localhost", 6379):
        missing.append("Redis (port 6379)")

    # Check pnpm
    if not shutil.which("pnpm"):
        missing.append("pnpm (npm package manager)")

    # Check git
    if not shutil.which("git"):
        missing.append("git")

    # Check Node.js
    if not shutil.which("node"):
        missing.append("node")

    return missing


def clone_firecrawl() -> bool:
    """Clone Firecrawl repo if not already present."""
    if FC_DIR.exists():
        print(f"Firecrawl repo already exists at {FC_DIR}")
        return True

    print(f"Cloning Firecrawl repo to {FC_DIR}...")
    print(f"  URL: {CLONE_URL}")

    try:
        result = subprocess.run(
            ["git", "clone", "--depth", "1", CLONE_URL, str(FC_DIR)],
            capture_output=True,
            text=True,
            timeout=300,
        )
        if result.returncode != 0:
            print(f"  ERROR: git clone failed: {result.stderr}")
            return False
        print("  Clone successful")
        return True
    except subprocess.TimeoutExpired:
        print("  ERROR: git clone timed out (5 min)")
        return False
    except Exception as e:
        print(f"  ERROR: {e}")
        return False


def install_deps() -> bool:
    """Install Firecrawl API dependencies."""
    if not FC_API_DIR.exists():
        print(f"  ERROR: Firecrawl API directory not found: {FC_API_DIR}")
        return False

    print(f"Installing Firecrawl dependencies in {FC_API_DIR}...")
    print("  Running: pnpm install")

    try:
        result = subprocess.run(
            ["pnpm", "install"],
            cwd=FC_API_DIR,
            capture_output=True,
            text=True,
            timeout=BUILD_TIMEOUT,
        )
        if result.returncode != 0:
            print(f"  ERROR: pnpm install failed")
            print(f"  stdout: {result.stdout[-500:] if result.stdout else '(empty)'}")
            print(f"  stderr: {result.stderr[-500:] if result.stderr else '(empty)'}")
            return False
        print("  Dependencies installed successfully")
        return True
    except subprocess.TimeoutExpired:
        print(f"  ERROR: pnpm install timed out ({BUILD_TIMEOUT}s)")
        return False
    except Exception as e:
        print(f"  ERROR: {e}")
        return False


def start_server() -> subprocess.Popen | None:
    """Start Firecrawl server. Returns process handle or None."""
    if not FC_API_DIR.exists():
        print("  ERROR: Firecrawl API directory not found")
        return None

    print(f"Starting Firecrawl server on port {FC_PORT}...")
    print("  Running: pnpm run start")

    env = os.environ.copy()
    env["PORT"] = str(FC_PORT)
    env["NODE_ENV"] = "development"

    try:
        proc = subprocess.Popen(
            ["pnpm", "run", "start"],
            cwd=FC_API_DIR,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            env=env,
        )
        return proc
    except Exception as e:
        print(f"  ERROR: Failed to start server: {e}")
        return None


def wait_for_healthy(timeout: int = STARTUP_TIMEOUT) -> bool:
    """Wait for Firecrawl to become healthy."""
    health_url = f"http://localhost:{FC_PORT}/health"
    print(f"Waiting for Firecrawl to become healthy (timeout: {timeout}s)...")

    start_time = time.time()
    while time.time() - start_time < timeout:
        if is_port_open("localhost", FC_PORT):
            try:
                import httpx

                resp = httpx.get(health_url, timeout=2)
                if resp.status_code == 200:
                    print(f"  Firecrawl is healthy on port {FC_PORT}")
                    return True
            except Exception:
                pass
        time.sleep(2)

    print(f"  ERROR: Firecrawl did not become healthy within {timeout}s")
    return False


def run_test_search() -> bool:
    """Run a test search query against Firecrawl."""
    import httpx

    search_url = f"http://localhost:{FC_PORT}/v1/search"
    test_query = "rust programming language"

    print(f"Running test search: '{test_query}'")

    try:
        import httpx

        resp = httpx.post(
            search_url,
            json={
                "query": test_query,
                "limit": 3,
                "scrapeOptions": {
                    "formats": ["markdown"],
                    "onlyMainContent": True,
                },
            },
            timeout=TEST_TIMEOUT,
        )

        if resp.status_code == 200:
            data = resp.json()
            results = data.get("data", {}).get("results", [])
            print(f"  SUCCESS: Got {len(results)} results")
            for i, r in enumerate(results[:2]):
                title = r.get("title", "(no title)")
                url = r.get("url", "(no url)")
                print(f"    [{i + 1}] {title}")
                print(f"        {url}")
            return True
        else:
            print(f"  ERROR: HTTP {resp.status_code}")
            print(f"  Response: {resp.text[:200]}")
            return False
    except Exception as e:
        print(f"  ERROR: {e}")
        return False


def stop_server(proc: subprocess.Popen):
    """Stop the Firecrawl server."""
    print("Stopping Firecrawl server...")
    proc.terminate()
    try:
        proc.wait(timeout=10)
        print("  Server stopped gracefully")
    except subprocess.TimeoutExpired:
        print("  Server did not stop, killing...")
        proc.kill()
        proc.wait()


def main():
    print("=" * 60)
    print("Firecrawl Bootstrap and Test Script")
    print("=" * 60)
    print()

    # Step 0: Check prerequisites
    print("Step 0: Checking prerequisites...")
    missing = check_prerequisites()
    if missing:
        print("  MISSING:")
        for item in missing:
            print(f"    - {item}")
        print()
        print("ERROR: Cannot proceed without prerequisites.")
        print("Install missing items and run again.")
        sys.exit(1)
    print("  All prerequisites available:")
    print("    ✓ PostgreSQL on port 5432")
    print("    ✓ Redis on port 6379")
    print("    ✓ pnpm installed")
    print("    ✓ git installed")
    print("    ✓ node installed")
    print()

    # Check if Firecrawl is already running
    if is_port_open("localhost", FC_PORT):
        print(f"Firecrawl is already running on port {FC_PORT}!")
        print("Running test against existing server...")
        success = run_test_search()
        if success:
            print("\n✓ All tests passed!")
        else:
            print("\n✗ Test failed!")
            sys.exit(1)
        return

    # Step 1: Clone Firecrawl
    print("Step 1: Clone Firecrawl repo...")
    if not clone_firecrawl():
        print("\n✗ Failed to clone Firecrawl!")
        sys.exit(1)
    print()

    # Step 2: Install dependencies
    print("Step 2: Install dependencies...")
    if not install_deps():
        print("\n✗ Failed to install dependencies!")
        sys.exit(1)
    print()

    # Step 3: Start server
    print("Step 3: Start Firecrawl server...")
    proc = start_server()
    if proc is None:
        print("\n✗ Failed to start server!")
        sys.exit(1)

    try:
        # Step 4: Wait for healthy
        print("Step 4: Wait for server to become healthy...")
        if not wait_for_healthy():
            print("\n✗ Server did not become healthy!")
            # Print any output from the server
            stderr = proc.stderr.read()
            if stderr:
                print(f"Server stderr: {stderr[-1000:]}")
            sys.exit(1)
        print()

        # Step 5: Run test
        print("Step 5: Run test search...")
        success = run_test_search()
        print()

        if success:
            print("✓ All tests passed!")
        else:
            print("✗ Test failed!")
            sys.exit(1)

    finally:
        # Step 6: Stop server
        print("Step 6: Stop server...")
        stop_server(proc)


if __name__ == "__main__":
    main()
