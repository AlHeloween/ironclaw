"""E2E tests for Local Code Search Service.

These tests verify the local code search service HTTP API by:
1. Building the local-code-search binary
2. Starting it with a temp config and index directory
3. Creating test indexes with sample code files
4. Verifying search and symbol-search endpoints return correct results
5. Verifying health and stats endpoints
6. Gracefully shutting down the service

Run with: pytest scenarios/test_local_code_search.py
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


LOCAL_CODE_SEARCH_DIR = (
    Path(__file__).parent.parent.parent / "src" / "local_code_search"
)
BINARY_PATH = LOCAL_CODE_SEARCH_DIR / "target" / "debug" / "local-code-search"
BUILD_TIMEOUT = 300
STARTUP_TIMEOUT = 30


def build_local_code_search():
    """Build the local-code-search binary if not already built."""
    if BINARY_PATH.exists():
        return BINARY_PATH

    print(f"Building local-code-search binary...")
    result = subprocess.run(
        ["cargo", "build"],
        cwd=LOCAL_CODE_SEARCH_DIR,
        capture_output=True,
        text=True,
        timeout=BUILD_TIMEOUT,
    )
    if result.returncode != 0:
        print(f"Build stdout: {result.stdout}")
        print(f"Build stderr: {result.stderr}")
        raise RuntimeError(f"Failed to build local-code-search: {result.stderr}")

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
def local_code_search_binary():
    """Session-scoped fixture: build the local-code-search binary."""
    return build_local_code_search()


@pytest.fixture()
def local_code_search_service(local_code_search_binary):
    """Start a local-code-search service with a temp config and index dir.

    Yields the base URL. Shuts down the service on teardown.
    """
    temp_dir = Path(tempfile.mkdtemp(prefix="e2e_lcs_"))
    try:
        index_dir = temp_dir / "indexes"
        index_dir.mkdir()

        config_dir = temp_dir / "config"
        config_dir.mkdir()
        config_file = config_dir / "local-code-search.jsonc"

        # Create a test codebase to index
        codebase = create_test_codebase(temp_dir)

        # Write config
        config_content = json.dumps(
            {
                "service": {
                    "port": 0,  # Auto-select port
                    "bind_address": "127.0.0.1",
                    "watch_enabled": False,
                },
                "indexes": [
                    {
                        "name": "test-project",
                        "path": str(codebase),
                        "languages": ["all"],
                        "symbols_enabled": True,
                        "enabled": True,
                    }
                ],
            }
        )
        config_file.write_text(config_content)

        # Start the service
        proc = subprocess.Popen(
            [
                str(local_code_search_binary),
                "--config",
                str(config_file),
            ],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )

        # Wait for service to start - poll health endpoint
        base_url = None
        start_time = time.time()
        while time.time() - start_time < STARTUP_TIMEOUT:
            # Read port from stdout if available, otherwise try common ports
            line = proc.stdout.readline()
            if "Starting Local Code Search Service" in line:
                # Extract port from log line
                import re

                match = re.search(r"on (\d+\.\d+\.\d+\.\d+):(\d+)", line)
                if match:
                    base_url = f"http://{match.group(1)}:{match.group(2)}"
                    break
            # Try default port
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

        # Wait for initial indexing to complete
        wait_for_ready(f"{base_url}/health", timeout=10)

        yield base_url

        # Shutdown
        proc.send_signal(signal.SIGINT)
        try:
            proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait()

    finally:
        # Cleanup temp directory
        shutil.rmtree(temp_dir, ignore_errors=True)


@pytest.mark.asyncio
async def test_health_endpoint(local_code_search_service):
    """Verify the health endpoint returns healthy status."""
    url = f"{local_code_search_service}/health"
    async with httpx.AsyncClient() as client:
        resp = await client.get(url)
        assert resp.status_code == 200
        data = resp.json()
        assert data["status"] == "healthy"
        assert data["service"] == "local-code-search"


@pytest.mark.asyncio
async def test_index_stats(local_code_search_service):
    """Verify the index stats endpoint returns index information."""
    url = f"{local_code_search_service}/index/stats"
    async with httpx.AsyncClient() as client:
        resp = await client.get(url)
        assert resp.status_code == 200
        data = resp.json()
        assert "indexes" in data
        assert "total_indexes" in data
        assert data["total_indexes"] >= 1
        # Should have our test-project index
        index_names = [idx["name"] for idx in data["indexes"]]
        assert "test-project" in index_names


@pytest.mark.asyncio
async def test_fulltext_search(local_code_search_service):
    """Verify full-text search returns results matching the query."""
    url = f"{local_code_search_service}/search"
    async with httpx.AsyncClient() as client:
        resp = await client.post(
            url,
            json={
                "query": "UserManager",
                "limit": 10,
            },
        )
        assert resp.status_code == 200
        data = resp.json()
        assert "results" in data
        assert "total" in data
        assert "query" in data
        assert data["query"] == "UserManager"
        assert data["total"] >= 1
        # Verify result structure
        result = data["results"][0]
        assert "title" in result
        assert "file_path" in result
        assert "language" in result
        assert result["language"] == "rust"


@pytest.mark.asyncio
async def test_fulltext_search_python(local_code_search_service):
    """Verify full-text search finds Python code."""
    url = f"{local_code_search_service}/search"
    async with httpx.AsyncClient() as client:
        resp = await client.post(
            url,
            json={
                "query": "UserService",
                "limit": 10,
            },
        )
        assert resp.status_code == 200
        data = resp.json()
        assert data["total"] >= 1
        # Find a result with Python language
        py_results = [r for r in data["results"] if r.get("language") == "python"]
        assert len(py_results) >= 1


@pytest.mark.asyncio
async def test_fulltext_search_typescript(local_code_search_service):
    """Verify full-text search finds TypeScript code."""
    url = f"{local_code_search_service}/search"
    async with httpx.AsyncClient() as client:
        resp = await client.post(
            url,
            json={
                "query": "UserStore",
                "limit": 10,
            },
        )
        assert resp.status_code == 200
        data = resp.json()
        assert data["total"] >= 1
        ts_results = [r for r in data["results"] if r.get("language") == "typescript"]
        assert len(ts_results) >= 1


@pytest.mark.asyncio
async def test_symbol_search(local_code_search_service):
    """Verify symbol search returns symbols matching the query."""
    url = f"{local_code_search_service}/symbol-search"
    async with httpx.AsyncClient() as client:
        resp = await client.post(
            url,
            json={
                "query": "createUser",
                "limit": 10,
            },
        )
        assert resp.status_code == 200
        data = resp.json()
        assert "results" in data
        assert data["total"] >= 1
        # Verify symbols are present in results
        result = data["results"][0]
        assert "symbols" in result
        assert len(result["symbols"]) >= 1
        # Check that at least one symbol matches
        symbol_names = [s["name"] for s in result["symbols"]]
        assert any(
            "createUser" in name or "create_user" in name for name in symbol_names
        )


@pytest.mark.asyncio
async def test_search_with_specific_index(local_code_search_service):
    """Verify search can target a specific index."""
    url = f"{local_code_search_service}/search"
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
        assert data["total"] >= 1
        # All results should be from the specified index
        for result in data["results"]:
            assert result["index_name"] == "test-project"


@pytest.mark.asyncio
async def test_search_empty_query_handling(local_code_search_service):
    """Verify search handles empty results gracefully."""
    url = f"{local_code_search_service}/search"
    async with httpx.AsyncClient() as client:
        resp = await client.post(
            url,
            json={
                "query": "xyznonexistent123",
                "limit": 10,
            },
        )
        assert resp.status_code == 200
        data = resp.json()
        assert data["total"] == 0
        assert data["results"] == []


@pytest.mark.asyncio
async def test_index_creation_via_api(local_code_search_service):
    """Verify new indexes can be created via the /index endpoint."""
    # Create a new temp codebase
    temp_dir = Path(tempfile.mkdtemp(prefix="e2e_lcs_api_"))
    try:
        new_codebase = temp_dir / "new-project"
        new_codebase.mkdir()
        (new_codebase / "hello.rs").write_text('fn hello() { println!("Hello"); }')

        url = f"{local_code_search_service}/index"
        async with httpx.AsyncClient() as client:
            resp = await client.post(
                url,
                json={
                    "name": "new-project",
                    "path": str(new_codebase),
                },
            )
            assert resp.status_code == 200
            data = resp.json()
            assert data["success"] is True
            assert "created and populated" in data["message"]

            # Verify the new index appears in stats
            stats_resp = await client.get(f"{local_code_search_service}/index/stats")
            stats = stats_resp.json()
            index_names = [idx["name"] for idx in stats["indexes"]]
            assert "new-project" in index_names

    finally:
        shutil.rmtree(temp_dir, ignore_errors=True)
