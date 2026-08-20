#!/usr/bin/env pwsh
# 完整重建：WASM → 复制到 dist → 重新打包 debug exe
#
# 用法：
#   .\rebuild.ps1              # 完整重建（WASM + dist + exe）
#   .\rebuild.ps1 -WasmOnly    # 只重建 WASM（dev 模式下 Ctrl+R 热重载）
param([switch]$WasmOnly)
$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

# 1. 编译 WASM
Write-Host "==> [1/3] Rebuilding pdf-viewer-ui wasm..." -ForegroundColor Cyan
npm run wasm:pdf-viewer-ui
if ($LASTEXITCODE -ne 0) {
    Write-Host "==> WASM build FAILED (exit $LASTEXITCODE)" -ForegroundColor Red
    exit $LASTEXITCODE
}

if ($WasmOnly) {
    Write-Host "==> Done. Press Ctrl+R in the Tauri window to reload." -ForegroundColor Green
    exit 0
}

# 2. 把新 WASM 复制到 dist/assets（vite build 不会自动做这一步）
Write-Host "==> [2/3] Copying WASM to dist/assets..." -ForegroundColor Cyan
$pkgWasm = "crates\pdf-viewer-ui\pkg\pdf_viewer_ui_bg.wasm"
if (!(Test-Path $pkgWasm)) {
    Write-Host "==> WASM not found at $pkgWasm" -ForegroundColor Red
    exit 1
}
$distWasm = Get-ChildItem -Path "dist\assets" -Filter "pdf_viewer_ui_bg-*.wasm" -ErrorAction SilentlyContinue | Select-Object -First 1
if ($distWasm) {
    Copy-Item $pkgWasm $distWasm.FullName -Force
    Write-Host "    Copied to $($distWasm.Name)" -ForegroundColor DarkGray
} else {
    Write-Host "    No existing WASM in dist/assets, vite build will handle it" -ForegroundColor DarkGray
}

# 3. 重新打包 debug exe
Write-Host "==> [3/3] Building debug exe..." -ForegroundColor Cyan
npx tauri build --debug --no-bundle
if ($LASTEXITCODE -ne 0) {
    Write-Host "==> Exe build FAILED (exit $LASTEXITCODE)" -ForegroundColor Red
    exit $LASTEXITCODE
}

Write-Host "==> All done. exe is at target\debug\pdf-viewer-standalone.exe" -ForegroundColor Green
