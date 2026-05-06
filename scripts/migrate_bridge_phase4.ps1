# Phase 4: Reorganize src/bridge/ into domain subdirectories.
# Run from repo root: pwsh scripts/migrate_bridge_phase4.ps1

$ErrorActionPreference = 'Stop'

$bridgeRoot = Join-Path $PSScriptRoot '..\src\bridge' | Resolve-Path | Select-Object -ExpandProperty Path

# File -> domain manifest. Files not listed stay at bridge root.
$manifest = [ordered]@{
    'editor_facade.ts'              = 'editor'
    'editor_wasm_api.ts'            = 'editor'
    'editor_host.ts'                = 'editor'
    'editor_host_diagnostics.ts'    = 'editor'
    'editor_host_view.ts'           = 'editor'

    'document_facade.ts'            = 'document'
    'document_edit_api.ts'          = 'document'
    'pdf_document_runtime.ts'       = 'document'

    'viewer_facade.ts'              = 'viewer'
    'viewer_session.ts'             = 'viewer'
    'viewer_geometry_probe.ts'      = 'viewer'
    'pdf_viewer_api.ts'             = 'viewer'
    'pdf_viewer_dom.ts'             = 'viewer'
    'pdf_runtime.ts'                = 'viewer'
    'pdf_keyboard.ts'               = 'viewer'

    'find_facade.ts'                = 'find'
    'find_facade_v2.ts'             = 'find'
    'pdf_find_controller.ts'        = 'find'

    'review_facade_v2.ts'           = 'review'
    'pdf_review_controller.ts'      = 'review'

    'comment_facade.ts'             = 'comment'
    'pdf_comment_contracts.ts'      = 'comment'
    'pdf_comment_controller.ts'     = 'comment'
    'pdf_comment_dom.ts'            = 'comment'
    'pdf_comment_host_actions.ts'   = 'comment'
    'pdf_comment_overlay_view.ts'   = 'comment'
    'pdf_comment_review_view.ts'    = 'comment'
    'pdf_comment_wasm_bridge.ts'    = 'comment'

    'render_facade.ts'              = 'render'
    'render_facade_v2.ts'           = 'render'
    'render_wasm_api.ts'            = 'render'
    'render_flow.ts'                = 'render'
    'frame_plan.ts'                 = 'render'
    'vector_canvas_host.ts'         = 'render'
    'vector_frame_cache.ts'         = 'render'
    'vector_host.ts'                = 'render'
    'vector_page_bundle.ts'         = 'render'
    'layout_trace.ts'               = 'render'

    'zoom_facade.ts'                = 'zoom'
    'zoom_controller.ts'            = 'zoom'

    'annotation_facade.ts'          = 'annotation'
    'pdf_annotation_controller.ts'  = 'annotation'

    'wasm_loader.ts'                = 'shared'
    'diagnostics.ts'                = 'shared'
}

# Build name -> new-relative-dir lookup (without trailing '/').
# Files staying at root map to '' (empty string).
$nameToDir = @{}
foreach ($k in $manifest.Keys) {
    $nameToDir[[IO.Path]::GetFileNameWithoutExtension($k)] = $manifest[$k]
}

# Existing ai/ files are already in subdir; record them too so imports survive.
$aiDir = Join-Path $bridgeRoot 'ai'
if (Test-Path $aiDir) {
    Get-ChildItem $aiDir -Filter *.ts | ForEach-Object {
        $nameToDir[[IO.Path]::GetFileNameWithoutExtension($_.Name)] = 'ai'
    }
}

function Get-RelImport {
    param(
        [string]$FromDomain,   # '' for bridge root
        [string]$ToDomain,     # '' for bridge root, or 'ai'/'editor'/...
        [string]$BaseName
    )
    if ($FromDomain -eq $ToDomain) {
        return "./$BaseName"
    }
    if ($FromDomain -eq '') {
        # from bridge/ -> bridge/<ToDomain>/
        return "./$ToDomain/$BaseName"
    }
    if ($ToDomain -eq '') {
        # from bridge/<FromDomain>/ -> bridge/
        return "../$BaseName"
    }
    # cross-domain
    return "../$ToDomain/$BaseName"
}

# 1. Create subdirs
$domains = @($manifest.Values | Sort-Object -Unique)
foreach ($d in $domains) {
    $p = Join-Path $bridgeRoot $d
    if (-not (Test-Path $p)) {
        New-Item -ItemType Directory -Path $p | Out-Null
        Write-Host "mkdir $p"
    }
}

# 2. Rewrite imports inside ALL .ts files under bridge (and main.ts) BEFORE moving.
#    We compute new path based on file's NEW location.
function Rewrite-File {
    param(
        [string]$Path,
        [string]$FromDomain  # this file's NEW domain (or '' for root, 'ai' for ai)
    )
    $orig = Get-Content -Raw -LiteralPath $Path
    $changed = $orig
    # Match: from './X'  OR  from "./X"  OR  import('./X')
    $pattern = "(['""])(\.\.?/[^'""]+)\1"
    $changed = [System.Text.RegularExpressions.Regex]::Replace(
        $changed,
        $pattern,
        {
            param($m)
            $quote = $m.Groups[1].Value
            $spec  = $m.Groups[2].Value
            # Only touch relative imports that originally pointed into bridge root
            # We need to detect: the original spec is relative; resolve target name.
            # For files moving from bridge root, '.' meant bridge root.
            # For files in subdirs after move, '.' means subdir.
            # Since rewrite happens BEFORE move, all current './X' references point to bridge root files.
            # ai/ files use '../X' which currently means bridge root.
            # Strategy: only rewrite if spec resolves to a known bridge file we know how to relocate.

            # Drop leading './' or '../' and any subdir
            $clean = $spec -replace '^\./','' -replace '^\.\./',''
            # If clean still has '/', it's e.g. 'ai/foo' or '../core/...'
            if ($clean -match '/') {
                # could be 'ai/X' from bridge root - keep as 'ai/X' but path may need updating
                if ($clean -match '^(ai)/(.+)$') {
                    $sub = $Matches[1]
                    $name = $Matches[2]
                    $new = Get-RelImport -FromDomain $FromDomain -ToDomain $sub -BaseName $name
                    return "$quote$new$quote"
                }
                return $m.Value  # leave non-bridge relative imports alone
            }
            $base = $clean
            if (-not $nameToDir.ContainsKey($base)) {
                return $m.Value  # unknown, leave
            }
            $toDomain = $nameToDir[$base]
            $new = Get-RelImport -FromDomain $FromDomain -ToDomain $toDomain -BaseName $base
            return "$quote$new$quote"
        }
    )
    if ($changed -ne $orig) {
        Set-Content -LiteralPath $Path -Value $changed -NoNewline:$false -Encoding UTF8
        Write-Host "rewrote $Path"
    }
}

# 2a. Rewrite each bridge file in place (using its NEW domain as FromDomain)
foreach ($f in Get-ChildItem $bridgeRoot -Filter *.ts) {
    $base = [IO.Path]::GetFileNameWithoutExtension($f.Name)
    $newDom = if ($nameToDir.ContainsKey($base)) { $nameToDir[$base] } else { '' }
    Rewrite-File -Path $f.FullName -FromDomain $newDom
}
foreach ($f in Get-ChildItem $aiDir -Filter *.ts) {
    Rewrite-File -Path $f.FullName -FromDomain 'ai'
}

# 2b. Rewrite main.ts (external caller). It uses './bridge/...' (absolute-ish under src/).
$mainTs = Join-Path $bridgeRoot '..\main.ts' | Resolve-Path | Select-Object -ExpandProperty Path
$mainOrig = Get-Content -Raw -LiteralPath $mainTs
$mainPattern = "(['""])\./bridge/([A-Za-z0-9_]+)\1"
$mainNew = [System.Text.RegularExpressions.Regex]::Replace(
    $mainOrig,
    $mainPattern,
    {
        param($m)
        $quote = $m.Groups[1].Value
        $base  = $m.Groups[2].Value
        if (-not $nameToDir.ContainsKey($base)) { return $m.Value }
        $dom = $nameToDir[$base]
        if ($dom -eq '') { return $m.Value }
        return "$quote./bridge/$dom/$base$quote"
    }
)
if ($mainNew -ne $mainOrig) {
    Set-Content -LiteralPath $mainTs -Value $mainNew -NoNewline:$false -Encoding UTF8
    Write-Host "rewrote $mainTs"
}

# 3. Move files
foreach ($k in $manifest.Keys) {
    $src = Join-Path $bridgeRoot $k
    if (-not (Test-Path $src)) { continue }
    $dst = Join-Path $bridgeRoot ($manifest[$k] + '\' + $k)
    Move-Item -LiteralPath $src -Destination $dst
    Write-Host "moved $k -> $($manifest[$k])/"
}

Write-Host "`nPhase 4 migration complete."
