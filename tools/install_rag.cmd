@echo off
setlocal enabledelayedexpansion
echo.
echo  =============================================
echo    ADID RAG 5.0.6 — Installer / Updater
echo  =============================================
echo.
echo  This script installs or updates ADID RAG
echo  including PyTorch, sentence-transformers,
echo  and the BGE embedding model.
echo.

REM Check Python
python --version >nul 2>&1
if !errorlevel! neq 0 (
    echo  [ERROR] Python 3.13 not found on PATH.
    echo.
    echo  Install Python 3.13 from https://python.org
    echo  Make sure "Add to PATH" is checked during install.
    echo.
    pause
    exit /b 1
)

echo  Python:
for /f "tokens=*" %%v in ('python --version 2^>^&1') do echo    %%v
echo.

REM Check torch / install
echo  Checking PyTorch...
python -c "import torch; print('CUDA:', torch.cuda.is_available())" 2>nul >nul
if !errorlevel! equ 0 (
    echo  PyTorch found.
    python -c "import torch; print('CUDA:', torch.cuda.is_available())"
) else (
    echo.
    echo  PyTorch not found. Installing CPU-only torch...
    echo  (For CUDA support, install CUDA toolkit first, then
    echo   pip install torch --index-url https://download.pytorch.org/whl/cu124)
    echo.
    pip install torch sentence-transformers
    if !errorlevel! neq 0 (
        echo.
        echo  [ERROR] Failed to install torch.
        echo  Try manually: pip install torch sentence-transformers
        pause
        exit /b 1
    )
    echo  PyTorch installed.
)

REM Install / upgrade ADID
echo.
echo  Installing ADID RAG 5.0.6...
REM Find the wheel relative to this script
set "SCRIPT_DIR=%~dp0"
set "WHEEL_DIR=%SCRIPT_DIR%..\dist"
if exist "%WHEEL_DIR%\adm-5.0.6-py3-none-any.whl" (
    echo  Installing from wheel: %WHEEL_DIR%\adm-5.0.6-py3-none-any.whl
    pip install --upgrade "%WHEEL_DIR%\adm-5.0.6-py3-none-any.whl[rag]"
) else (
    echo  Wheel not found, installing from PyPI...
    pip install --upgrade "adm[rag]>5.0"
)
if !errorlevel! neq 0 (
    echo.
    echo  [ERROR] pip install failed.
    echo  Try: pip install adm[rag]
    pause
    exit /b 1
)

REM Verify
echo.
echo  Verifying installation...
echo.
adm-rag --init

echo.
echo  =============================================
echo    ADID RAG installed successfully.
echo.
echo    Usage:
echo      adm --rag index ^<name^> .
echo      adm --query ^<name^> "your question"
echo      adm --mcp-http 127.0.0.1 7990   (model daemon)
echo  =============================================
echo.
pause
