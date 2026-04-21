"""E2E tests for Universal Search Service.

These tests verify the universal search service HTTP API by:
1. Building the universal-search-service binary
2. Starting it with a temp config and index directory
3. Creating test indexes with sample code files
4. Verifying search and symbol-search endpoints return correct results
5. Verifying health and stats endpoints
6. Gracefully shutting down the service

Run with: pytest scenarios/test_universal_search.py
"""

import json
import os
import shutil
import signal
import subprocess
import sys
import tempfile
import time
from pathlib import Path

import httpx
import pytest

from helpers import wait_for_ready


UNIVERSAL_SEARCH_DIR = Path(__file__).parent.parent.parent.parent / "universal_search"
BINARY_PATH = (
    Path(__file__).parent.parent.parent.parent
    / "target"
    / "debug"
    / "universal-search-service"
)
BUILD_TIMEOUT = 300
STARTUP_TIMEOUT = 30


def build_universal_search():
    """Build the universal-search-service binary if not already built."""
    if BINARY_PATH.exists():
        return BINARY_PATH

    print(f"Building universal-search-service binary...")
    workspace_root = Path(__file__).parent.parent.parent.parent
    result = subprocess.run(
        ["cargo", "build", "--package", "universal-search-service"],
        cwd=workspace_root,
        capture_output=True,
        text=True,
        timeout=BUILD_TIMEOUT,
    )
    if result.returncode != 0:
        print(f"Build stdout: {result.stdout}")
        print(f"Build stderr: {result.stderr}")
        raise RuntimeError(f"Failed to build universal-search-service: {result.stderr}")

    assert BINARY_PATH.exists(), f"Binary not found at {BINARY_PATH}"
    return BINARY_PATH


def create_test_codebase(temp_dir: Path) -> Path:
    """Create a small test codebase with Rust, Python, and TypeScript files."""
    codebase = temp_dir / "test-project"
    codebase.mkdir(parents=True)

    # Rust file
    rust_file = codebase / "lib.rs"
    rust_file.write_text("""
pub struct UserManager {
    pub name: String,
}

pub fn create_user(name: String) -> UserManager {
    UserManager { name }
}

pub trait Greeter {
    fn greet(&self) -> String;
}

pub enum Status {
    Active,
    Inactive,
}

fn main() {
    println!("Hello from test project!");
}
""")

    # Python file
    py_file = codebase / "service.py"
    py_file.write_text("""
class UserService:
    def __init__(self):
        self.users = []

    def add_user(self, name):
        self.users.append(name)

    def get_users(self):
        return self.users

def process_data(data):
    return [x.upper() for x in data]
""")

    # TypeScript file
    ts_file = codebase / "app.ts"
    ts_file.write_text("""
export interface User {
    name: string;
    id: number;
}

export class UserStore {
    private users: User[] = [];

    addUser(user: User): void {
        this.users.push(user);
    }

    getUsers(): User[] {
        return this.users;
    }
}

export function createUser(name: string, id: number): User {
    return { name, id };
}
""")

    return codebase


@pytest.fixture(scope="session")
def universal_search_binary():
    """Session-scoped fixture: build the universal-search-service binary."""
    return build_universal_search()


@pytest.fixture()
def universal_search_service(universal_search_binary):
    """Start a universal-search-service with a temp config and index dir.

    Yields the base URL (port 3005 for web search, port 3004 for local search).
    Shuts down the service on teardown.
    """
    temp_dir = Path(tempfile.mkdtemp(prefix="e2e_us_"))
    try:
        index_dir = temp_dir / "indexes"
        index_dir.mkdir()

        config_dir = temp_dir / "config"
        config_dir.mkdir()
        config_file = config_dir / "universal-search.jsonc"

        # Create a test codebase to index
        codebase = create_test_codebase(temp_dir)

        # Write config
        config_content = json.dumps(
            {
                "service": {
                    "port": 0,
                    "bind_address": "127.0.0.1",
                },
                "local_search": {
                    "enabled": True,
                    "port": 3004,
                    "watch_enabled": False,
                    "indexes_dir": str(index_dir),
                    "indexes": [
                        {
                            "name": "test-project",
                            "path": str(codebase),
                            "languages": ["all"],
                            "symbols_enabled": True,
                            "enabled": True,
                        }
                    ],
                },
                "web_search": {
                    "firecrawl": {
                        "api_url": "http://localhost:3002",
                        "source": "local",
                        "auto_start": False,
                    },
                    "sourcegraph": {
                        "access_token": None,
                    },
                },
                "bootstrap": {
                    "auto_clone_firecrawl": False,
                    "auto_install_deps": False,
                },
            }
        )
        config_file.write_text(config_content)

        # Start the service
        proc = subprocess.Popen(
            [
                str(universal_search_binary),
                "run",
                "--config",
                str(config_file),
                "--log-level",
                "warn",
            ],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )

        # Wait for service to start
        base_url = None
        start_time = time.time()
        while time.time() - start_time < STARTUP_TIMEOUT:
            for port in range(3004, 3010):
                try:
                    resp = httpx.get(f"http://127.0.0.1:{port}/health", timeout=1.0)
                    if resp.status_code == 200:
                        base_url = f"http://127.0.0.1:{port}"
                        break
                except Exception:
                    continue
            if base_url:
                break
            time.sleep(0.5)

        if not base_url:
            proc.kill()
            stderr = proc.stderr.read()
            raise RuntimeError(
                f"Service failed to start within {STARTUP_TIMEOUT}s. Stderr: {stderr}"
            )

        wait_for_ready(f"{base_url}/health", timeout=10)

        yield base_url

        proc.send_signal(signal.SIGINT)
        try:
            proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait()

    finally:
        shutil.rmtree(temp_dir, ignore_errors=True)


@pytest.mark.asyncio
async def test_health_endpoint(universal_search_service):
    """Verify the health endpoint returns healthy status."""
    url = f"{universal_search_service}/health"
    async with httpx.AsyncClient() as client:
        resp = await client.get(url)
        assert resp.status_code == 200
        data = resp.json()
        assert data["status"] == "healthy"
        assert data["service"] == "universal-search-service"


@pytest.mark.asyncio
async def test_index_stats(universal_search_service):
    """Verify the index stats endpoint returns index information."""
    url = f"{universal_search_service}/local/index/stats"
    async with httpx.AsyncClient() as client:
        resp = await client.get(url)
        assert resp.status_code == 200
        data = resp.json()
        assert "indexes" in data
        index_names = [idx["name"] for idx in data["indexes"]]
        assert "test-project" in index_names


@pytest.mark.asyncio
async def test_fulltext_search(universal_search_service):
    """Verify full-text search returns results matching the query."""
    url = f"{universal_search_service}/local/search"
    async with httpx.AsyncClient() as client:
        resp = await client.post(
            url,
            json={
                "query": "UserManager",
                "index": "test-project",
                "limit": 10,
            },
        )
        assert resp.status_code == 200
        data = resp.json()
        assert len(data) >= 1
        total_results = sum(r.get("total", 0) for r in data)
        assert total_results >= 1


@pytest.mark.asyncio
async def test_fulltext_search_python(universal_search_service):
    """Verify full-text search finds Python code."""
    url = f"{universal_search_service}/local/search"
    async with httpx.AsyncClient() as client:
        resp = await client.post(
            url,
            json={
                "query": "UserService",
                "index": "test-project",
                "limit": 10,
            },
        )
        assert resp.status_code == 200
        data = resp.json()
        total_results = sum(r.get("total", 0) for r in data)
        assert total_results >= 1


@pytest.mark.asyncio
async def test_fulltext_search_typescript(universal_search_service):
    """Verify full-text search finds TypeScript code."""
    url = f"{universal_search_service}/local/search"
    async with httpx.AsyncClient() as client:
        resp = await client.post(
            url,
            json={
                "query": "UserStore",
                "index": "test-project",
                "limit": 10,
            },
        )
        assert resp.status_code == 200
        data = resp.json()
        total_results = sum(r.get("total", 0) for r in data)
        assert total_results >= 1


@pytest.mark.asyncio
async def test_symbol_search(universal_search_service):
    """Verify symbol search returns symbols matching the query."""
    url = f"{universal_search_service}/local/symbol-search"
    async with httpx.AsyncClient() as client:
        resp = await client.post(
            url,
            json={
                "query": "createUser",
                "index": "test-project",
            },
        )
        assert resp.status_code == 200
        data = resp.json()
        assert "total" in data
        assert data["total"] >= 1


@pytest.mark.asyncio
async def test_search_empty_query_handling(universal_search_service):
    """Verify search handles empty results gracefully."""
    url = f"{universal_search_service}/local/search"
    async with httpx.AsyncClient() as client:
        resp = await client.post(
            url,
            json={
                "query": "xyznonexistent123",
                "index": "test-project",
                "limit": 10,
            },
        )
        assert resp.status_code == 200
        data = resp.json()
        total_results = sum(r.get("total", 0) for r in data)
        assert total_results == 0


@pytest.mark.asyncio
async def test_index_creation_via_api(universal_search_service):
    """Verify new indexes can be created via the /local/index endpoint."""
    temp_dir = Path(tempfile.mkdtemp(prefix="e2e_us_api_"))
    try:
        new_codebase = temp_dir / "new-project"
        new_codebase.mkdir()
        (new_codebase / "hello.rs").write_text('fn hello() { println!("Hello"); }')

        url = f"{universal_search_service}/local/index"
        async with httpx.AsyncClient() as client:
            resp = await client.post(
                url,
                json={
                    "name": "new-project",
                    "path": str(new_codebase),
                    "languages": ["rust"],
                    "symbols_enabled": True,
                },
            )
            assert resp.status_code == 200
            data = resp.json()
            assert data["status"] == "created"

            stats_resp = await client.get(
                f"{universal_search_service}/local/index/stats"
            )
            stats = stats_resp.json()
            index_names = [idx["name"] for idx in stats["indexes"]]
            assert "new-project" in index_names

    finally:
        shutil.rmtree(temp_dir, ignore_errors=True)
