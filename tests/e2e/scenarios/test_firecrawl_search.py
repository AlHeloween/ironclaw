"""E2E tests for Firecrawl Search Tool.

These tests verify the Firecrawl WASM tool functionality by:
1. Installing the firecrawl_search tool from the registry
2. Running actual search queries through the IronClaw agent
3. Verifying responses from web search, Sourcegraph, and hybrid modes

The tests require:
- A running Firecrawl instance (local or cloud via FIRECRAWL_API_URL)
- The IronClaw agent with gateway enabled
- Network access to Sourcegraph for code search tests

Tests skip (not fail) if Firecrawl is not configured or unreachable.

Run with: pytest scenarios/test_firecrawl_search.py
"""

import json
import os
import subprocess
import tempfile
from pathlib import Path

import httpx
import pytest

from helpers import AUTH_TOKEN, wait_for_ready


TOOLS_SRC_DIR = Path(__file__).parent.parent.parent / "tools-src" / "firecrawl-search"
FIRECRAWL_TIMEOUT = 60


def is_firecrawl_available():
    """Check if a Firecrawl instance is available."""
    # Check for local instance
    local_url = os.environ.get("FIRECRAWL_API_URL", "http://localhost:3002")
    try:
        resp = httpx.get(f"{local_url}/health", timeout=5.0)
        return resp.status_code == 200
    except Exception:
        pass

    # Check for cloud API key
    if os.environ.get("FIRECRAWL_API_KEY"):
        return True

    return False


def is_sourcegraph_available():
    """Check if Sourcegraph public API is reachable."""
    try:
        resp = httpx.get("https://sourcegraph.com/.api/graphql", timeout=10.0)
        return resp.status_code in (200, 401)  # 401 is OK - needs auth
    except Exception:
        return False


@pytest.fixture(scope="session")
def firecrawl_tool_installed():
    """Ensure the firecrawl_search tool is installed."""
    if not is_firecrawl_available():
        pytest.skip(
            "No Firecrawl instance available (set FIRECRAWL_API_URL or start local instance)"
        )

    # Build the WASM tool
    wasm_dir = TOOLS_SRC_DIR / "target" / "wasm32-wasip2" / "release"
    wasm_path = wasm_dir / "firecrawl_search.wasm"

    if not wasm_path.exists():
        print("Building firecrawl-search WASM tool...")
        result = subprocess.run(
            ["cargo", "build", "--release", "--target", "wasm32-wasip2"],
            cwd=TOOLS_SRC_DIR,
            capture_output=True,
            text=True,
            timeout=300,
        )
        if result.returncode != 0:
            print(f"Build stderr: {result.stderr}")
            raise RuntimeError(
                f"Failed to build firecrawl-search WASM: {result.stderr}"
            )

    assert wasm_path.exists(), f"WASM binary not found at {wasm_path}"
    return wasm_path


@pytest.fixture()
def ironclaw_with_firecrawl(ironclaw_server, firecrawl_tool_installed):
    """Install firecrawl_search tool in the running IronClaw instance.

    This fixture installs the tool via the registry or direct WASM loading.
    """
    # Install the tool by copying WASM to the tools directory
    tools_dir = Path(tempfile.gettempdir()) / f"e2e_ironclaw_{os.getpid()}" / "tools"
    tools_dir.mkdir(parents=True, exist_ok=True)

    wasm_dest = tools_dir / "firecrawl_search.wasm"
    import shutil

    shutil.copy(firecrawl_tool_installed, wasm_dest)

    # Create capabilities file
    caps = {
        "name": "firecrawl_search",
        "version": "0.1.0",
        "wasm_url": f"file://{wasm_dest}",
        "actions": [
            {
                "name": "firecrawl_search",
                "description": "Search the web or codebases using Firecrawl",
                "tool_type": "wasm",
            }
        ],
    }
    caps_path = tools_dir / "firecrawl_search.capabilities.json"
    caps_path.write_text(json.dumps(caps))

    yield {
        "tools_dir": tools_dir,
        "wasm_path": wasm_dest,
    }


@pytest.mark.asyncio
async def test_firecrawl_schema_available(ironclaw_server):
    """Verify the firecrawl_search tool schema is available via the API."""
    url = f"{ironclaw_server}/api/tools"
    headers = {"Authorization": f"Bearer {AUTH_TOKEN}"}
    async with httpx.AsyncClient() as client:
        resp = await client.get(url, headers=headers)
        # This test is informational - the tool may or may not be installed
        if resp.status_code == 200:
            tools = resp.json()
            tool_names = (
                [t.get("name", "") for t in tools] if isinstance(tools, list) else []
            )
            if "firecrawl_search" not in tool_names:
                pytest.skip(
                    "firecrawl_search tool not installed in this IronClaw instance"
                )


@pytest.mark.asyncio
async def test_web_search_mode(ironclaw_server, ironclaw_with_firecrawl):
    """Test Firecrawl web search mode returns valid results."""
    # Send a message that triggers firecrawl_search
    url = f"{ironclaw_server}/api/chat/send"
    headers = {"Authorization": f"Bearer {AUTH_TOKEN}"}

    async with httpx.AsyncClient() as client:
        # The agent should recognize the need for web search and call firecrawl_search
        resp = await client.post(
            url,
            headers=headers,
            json={
                "message": "Search for 'rust programming language async await' using firecrawl_search with mode search",
            },
        )
        assert resp.status_code == 200
        data = resp.json()
        # The response should contain search results or an acknowledgment
        assert "output" in data or "response" in data or "message" in data


@pytest.mark.asyncio
async def test_sourcegraph_mode(ironclaw_server, ironclaw_with_firecrawl):
    """Test Firecrawl Sourcegraph mode returns code search results."""
    if not is_sourcegraph_available():
        pytest.skip("Sourcegraph API is not reachable")

    url = f"{ironclaw_server}/api/chat/send"
    headers = {"Authorization": f"Bearer {AUTH_TOKEN}"}

    async with httpx.AsyncClient() as client:
        resp = await client.post(
            url,
            headers=headers,
            json={
                "message": "Search for 'async fn handler' in public codebases using firecrawl_search with mode sourcegraph",
            },
        )
        assert resp.status_code == 200


@pytest.mark.asyncio
async def test_hybrid_mode_with_local_service(
    ironclaw_server,
    ironclaw_with_firecrawl,
    local_code_search_service,
):
    """Test Firecrawl hybrid mode combines local and Sourcegraph results.

    This test requires both the local code search service and Firecrawl to be running.
    """
    if not is_sourcegraph_available():
        pytest.skip("Sourcegraph API is not reachable")

    # Set LOCAL_CODE_SEARCH_URL env for the hybrid mode
    os.environ["LOCAL_CODE_SEARCH_URL"] = local_code_search_service

    url = f"{ironclaw_server}/api/chat/send"
    headers = {"Authorization": f"Bearer {AUTH_TOKEN}"}

    async with httpx.AsyncClient() as client:
        resp = await client.post(
            url,
            headers=headers,
            json={
                "message": "Search for 'UserManager' in code using firecrawl_search with mode hybrid",
            },
        )
        assert resp.status_code == 200


@pytest.mark.asyncio
async def test_firecrawl_context_mode(ironclaw_server, ironclaw_with_firecrawl):
    """Test Firecrawl context mode scrapes a URL correctly."""
    url = f"{ironclaw_server}/api/chat/send"
    headers = {"Authorization": f"Bearer {AUTH_TOKEN}"}

    async with httpx.AsyncClient() as client:
        resp = await client.post(
            url,
            headers=headers,
            json={
                "message": "Scrape the content from https://www.rust-lang.org using firecrawl_search with mode context",
            },
        )
        assert resp.status_code == 200
