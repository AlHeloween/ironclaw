import shutil
import socket

import pytest
import requests

from conftest import BASE_URL


class TestHealth:
    def test_health_endpoint(self, http_client, service_process):
        r = http_client.get(f"{BASE_URL}/health", timeout=5)
        assert r.status_code == 200
        body = r.json()
        assert body["status"] == "healthy"
        assert body["service"] == "universal-search-service"

    def test_status_endpoint(self, http_client, service_process):
        r = http_client.get(f"{BASE_URL}/status", timeout=5)
        assert r.status_code == 200
        body = r.json()
        assert body["status"] == "healthy"


def _is_port_open(host: str, port: int) -> bool:
    try:
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            s.settimeout(2)
            s.connect((host, port))
            return True
    except (OSError, socket.timeout):
        return False


def _diagnose_firecrawl_failure(http_client):
    """Return a detailed diagnostic report explaining why Firecrawl is unavailable."""
    reasons = []

    fc_port_open = _is_port_open("localhost", 3002)
    if not fc_port_open:
        reasons.append("Firecrawl not listening on localhost:3002")

    pg_open = _is_port_open("localhost", 5432)
    if not pg_open:
        reasons.append(
            "PostgreSQL not listening on localhost:5432 (Firecrawl dependency)"
        )

    redis_open = _is_port_open("localhost", 6379)
    if not redis_open:
        reasons.append("Redis not listening on localhost:6379 (Firecrawl dependency)")

    if not shutil.which("pnpm"):
        reasons.append("pnpm not installed (needed to install Firecrawl dependencies)")

    r = http_client.get(f"{BASE_URL}/status", timeout=5)
    if r.status_code == 200:
        status = r.json()
        ws = status.get("web_search", {})
        if not ws.get("healthy", True):
            reasons.append(f"Web search unhealthy: {ws}")

    import os

    if not os.environ.get("FIRECRAWL_API_KEY"):
        reasons.append("FIRECRAWL_API_KEY env var not set (needed for cloud Firecrawl)")

    if not reasons:
        reasons.append("Unknown reason (Firecrawl port open but request failed)")

    return "; ".join(reasons)


def _is_firecrawl_error(r):
    if r.status_code in (502, 503):
        return True
    if r.status_code == 500:
        try:
            body = r.json()
            text = body.get("error", "").lower()
            if any(kw in text for kw in ["connect", "connection", "refused", "tcp"]):
                return True
        except Exception:
            pass
    return False


class TestWebSearch:
    """Web search tests — require a running Firecrawl instance with PostgreSQL + Redis.

    These tests FAIL (not skip) when Firecrawl is unavailable, with full diagnostic
    details showing exactly what conditions need to be met.
    """

    def _fail_with_diagnostics(self, http_client, query):
        reason = _diagnose_firecrawl_failure(http_client)
        pytest.fail(
            f"Web search test FAILED for query '{query}'. "
            f"Firecrawl is not available. Until all conditions are completed, "
            f"these tests are absolutely failed.\n"
            f"Diagnostics: {reason}"
        )

    def test_web_search_current_time(self, http_client, service_process):
        from datetime import datetime

        now = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
        payload = {"query": f"current time {now}", "count": 3}
        r = http_client.post(f"{BASE_URL}/web/search", json=payload, timeout=60)

        if r.status_code == 200:
            body = r.json()
            assert body["query"] == f"current time {now}"
            assert body["mode"] == "search"
            assert isinstance(body.get("result_count"), int)
            assert body["result_count"] >= 0
            for result in body.get("results", []):
                assert "title" in result
                assert "url" in result
            return

        if _is_firecrawl_error(r):
            self._fail_with_diagnostics(http_client, f"current time {now}")

        pytest.fail(f"Unexpected response (status {r.status_code}): {r.text}")

    def test_web_search_rust_codebases(self, http_client, service_process):
        payload = {"query": "rust codebase examples github", "count": 5}
        r = http_client.post(f"{BASE_URL}/web/search", json=payload, timeout=60)

        if r.status_code == 200:
            body = r.json()
            assert body["mode"] == "search"
            assert body["result_count"] >= 0
            for result in body.get("results", []):
                assert "url" in result
            return

        if _is_firecrawl_error(r):
            self._fail_with_diagnostics(http_client, "rust codebase examples github")

        pytest.fail(f"Unexpected response (status {r.status_code}): {r.text}")


class TestLocalSearch:
    """Local indexed codebase search — uses a small test directory."""

    def test_create_small_index(self, http_client, service_process, tmp_path):
        test_dir = tmp_path / "test_code"
        test_dir.mkdir()
        (test_dir / "hello.rs").write_text(
            "pub struct Config { pub name: String }\n"
            "impl Config { pub fn new() -> Self { Config { name: String::new() } } }\n"
        )
        (test_dir / "main.rs").write_text("fn main() { let cfg = Config::new(); }\n")

        payload = {
            "name": "small-test",
            "path": str(test_dir),
            "languages": ["rust"],
            "symbols_enabled": True,
        }
        r = http_client.post(f"{BASE_URL}/local/index", json=payload, timeout=60)
        assert r.status_code == 200, f"Failed to create index: {r.text}"
        body = r.json()
        assert body["name"] == "small-test"
        assert body["status"] == "created"

    def test_index_stats(self, http_client, service_process):
        r = http_client.get(f"{BASE_URL}/local/index/stats", timeout=10)
        assert r.status_code == 200
        body = r.json()
        assert "indexes" in body
        names = [idx["name"] for idx in body["indexes"]]
        assert "small-test" in names

    def test_local_fulltext_search(self, http_client, service_process):
        payload = {
            "query": "struct Config",
            "index": "small-test",
            "limit": 5,
        }
        r = http_client.post(f"{BASE_URL}/local/search", json=payload, timeout=30)
        assert r.status_code == 200, f"Search failed: {r.text}"
        body = r.json()
        assert len(body) > 0
        total_results = sum(resp.get("total", 0) for resp in body)
        assert total_results > 0, "No results found for 'struct Config'"

        for resp in body:
            for result in resp.get("results", []):
                assert "file_path" in result
                assert "content" in result
                assert "score" in result

    def test_local_symbol_search(self, http_client, service_process):
        payload = {
            "query": "Config",
            "index": "small-test",
        }
        r = http_client.post(
            f"{BASE_URL}/local/symbol-search", json=payload, timeout=30
        )
        assert r.status_code == 200, f"Symbol search failed: {r.text}"
        body = r.json()
        assert "total" in body
        assert body["total"] > 0, "No symbol results for 'Config'"


class TestFirecrawlDiagnostics:
    """Extended diagnostics: report Firecrawl availability status explicitly."""

    def test_firecrawl_availability_report(self, http_client, service_process):
        """Always passes but reports detailed Firecrawl status in the test output."""
        report = []
        report.append("=== Firecrawl Availability Report ===")
        report.append("")
        report.append("REQUIRED CONDITIONS FOR WEB SEARCH TESTS:")

        fc_port = _is_port_open("localhost", 3002)
        report.append(
            f"  [ {'PASS' if fc_port else 'FAIL'} ] Firecrawl listening on localhost:3002"
        )

        pg_port = _is_port_open("localhost", 5432)
        report.append(
            f"  [ {'PASS' if pg_port else 'FAIL'} ] PostgreSQL listening on localhost:5432"
        )

        redis_port = _is_port_open("localhost", 6379)
        report.append(
            f"  [ {'PASS' if redis_port else 'FAIL'} ] Redis listening on localhost:6379"
        )

        has_pnpm = bool(shutil.which("pnpm"))
        report.append(f"  [ {'PASS' if has_pnpm else 'FAIL'} ] pnpm installed")

        import os

        has_key = bool(os.environ.get("FIRECRAWL_API_KEY"))
        report.append(
            f"  [ {'PASS' if has_key else 'FAIL'} ] FIRECRAWL_API_KEY env var set"
        )

        r = http_client.get(f"{BASE_URL}/status", timeout=5)
        if r.status_code == 200:
            status = r.json()
            ws = status.get("web_search", {})
            ws_healthy = ws.get("healthy", False)
            report.append(
                f"  [ {'PASS' if ws_healthy else 'FAIL'} ] Web search service healthy"
            )

        report.append("")
        report.append("Service status:")
        if r.status_code == 200:
            report.append(f"  {status}")

        all_pass = fc_port and pg_port and redis_port and has_pnpm and has_key
        if all_pass:
            report.append("")
            report.append("STATUS: All conditions met — web search tests should pass.")
        else:
            report.append("")
            report.append("STATUS: NOT ALL conditions met.")
            report.append(
                "Until all conditions are completed, web search tests are ABSOLUTELY FAILED."
            )

        report.append("=====================================")
        for line in report:
            print(line)

        assert True
