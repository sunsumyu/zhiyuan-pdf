@echo off
REM 一键重新编译 wasm 模块
cd /d %~dp0
echo ==^> Rebuilding pdf-viewer-ui wasm...
call npm run wasm:pdf-viewer-ui
if %ERRORLEVEL% NEQ 0 (
    echo ==^> Build FAILED
    exit /b %ERRORLEVEL%
)
echo ==^> Done. Press Ctrl+R in the Tauri window to reload.
