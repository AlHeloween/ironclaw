#!/usr/bin/env bash
set -euo pipefail

echo
echo "  ============================================="
echo "    ADID RAG 5.0.6 — Installer / Updater"
echo "  ============================================="
echo

# Check Python
if ! command -v python3.13 &>/dev/null && ! command -v python3 &>/dev/null; then
    echo "  [ERROR] Python 3.13 not found."
    echo
    echo "  Install Python 3.13 from https://python.org"
    echo "  or via your system package manager."
    exit 1
fi

PY=$(command -v python3.13 2>/dev/null || command -v python3 2>/dev/null)
echo "  Python: $($PY --version)"
echo

# Check / install torch
echo "  Checking PyTorch..."
if $PY -c "import torch" 2>/dev/null; then
    echo "  PyTorch found."
    $PY -c "import torch; print('  CUDA:', torch.cuda.is_available())" 2>/dev/null || true
else
    echo "  PyTorch not found. Installing CPU-only torch..."
    $PY -m pip install torch sentence-transformers
fi

# Install / upgrade ADID
echo
echo "  Installing ADID RAG 5.0.6..."
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
WHEEL_DIR="$SCRIPT_DIR/../dist"
WHEEL=$(ls "$WHEEL_DIR"/adm-5.0.6-py3-none-any.whl 2>/dev/null | head -1)

if [ -n "${WHEEL:-}" ] && [ -f "$WHEEL" ]; then
    echo "  Installing from wheel: $WHEEL"
    $PY -m pip install --upgrade "$WHEEL[rag]"
else
    echo "  Wheel not found, installing from PyPI..."
    $PY -m pip install --upgrade "adm[rag]>=5.0"
fi

# Verify
echo
echo "  Verifying..."
$PY -m adm --init 2>&1 | head -5 || true

echo
echo "  ============================================="
echo "    Installed. Usage:"
echo "      adm --rag index <name> ."
echo "      adm --query <name> 'your question'"
echo "      adm --mcp-http 127.0.0.1 7990"
echo "  ============================================="
echo
