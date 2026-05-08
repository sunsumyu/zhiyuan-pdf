#!/usr/bin/env pwsh
# 一键重新编译 wasm 模块（tauri 在 watch 模式下会自动热重载）
$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot
Write-Host "==> Rebuilding pdf-viewer-ui wasm..." -ForegroundColor Cyan
npm run wasm:pdf-viewer-ui
if ($LASTEXITCODE -eq 0) {
    Write-Host "==> Done. Press Ctrl+R in the Tauri window to reload." -ForegroundColor Green
} else {
    Write-Host "==> Build FAILED (exit $LASTEXITCODE)" -ForegroundColor Red
    exit $LASTEXITCODE
}
