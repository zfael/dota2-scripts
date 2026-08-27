<#
.SYNOPSIS
    Downloads the hero catalog and landscape portraits used to seed draft fingerprints.

.DESCRIPTION
    Source of truth is https://github.com/odota/dotaconstants, the same catalog
    generate-mana-costs.ps1 already uses. Its heroes.json carries an `img` path
    per hero pointing at Valve's CDN; this fetches the catalog, then each
    portrait.

    These portraits are only a *bootstrap* for hero matching. Dota renders draft
    slots with its own crop, scale, and colour grading, so a CDN portrait never
    matches a live capture exactly. The runtime is expected to replace each
    reference with a real captured crop once a hero is confirmed. Seeding from
    here just means matching works before that has happened.

    Portraits land outside the repo tree in .cache/ (gitignored) because they are
    ~127 binary files that go stale whenever Valve adds a hero.

.NOTES
    Forces IPv4. On at least one dev machine, HTTPS over IPv6 fails the TLS
    handshake against these hosts while IPv4 succeeds; without -4 every request
    dies with a schannel error that looks like a certificate problem.
#>
[CmdletBinding()]
param(
    [string]$CacheDir = (Join-Path $PSScriptRoot "..\.cache\dotaconstants"),
    [string]$PortraitDir = (Join-Path $PSScriptRoot "..\.cache\hero_portraits"),
    [switch]$Force
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

$ConstantsUrl = "https://raw.githubusercontent.com/odota/dotaconstants/master/build"
$CdnBase = "https://cdn.cloudflare.steamstatic.com"

function Invoke-Download {
    param([string]$Uri, [string]$OutFile)

    # curl.exe rather than Invoke-WebRequest: -4 is the whole point (see NOTES),
    # and Invoke-WebRequest has no equivalent switch.
    $code = & curl.exe -4 -s -L --max-time 60 -o $OutFile -w "%{http_code}" $Uri
    if ($LASTEXITCODE -ne 0) {
        throw "curl failed (exit $LASTEXITCODE) for $Uri"
    }
    if ($code -ne "200") {
        throw "HTTP $code for $Uri"
    }
}

New-Item -ItemType Directory -Force -Path $CacheDir | Out-Null
New-Item -ItemType Directory -Force -Path $PortraitDir | Out-Null

# --- Hero catalog -----------------------------------------------------------
$heroesJson = Join-Path $CacheDir "heroes.json"
if ($Force -or -not (Test-Path $heroesJson)) {
    Write-Host "Fetching heroes.json..." -NoNewline
    Invoke-Download -Uri "$ConstantsUrl/heroes.json" -OutFile $heroesJson
    Write-Host " done"
} else {
    Write-Host "heroes.json cached (use -Force to refresh)"
}

$heroes = (Get-Content $heroesJson -Raw | ConvertFrom-Json).PSObject.Properties | ForEach-Object { $_.Value }
Write-Host "Catalog lists $($heroes.Count) heroes."

# --- Portraits --------------------------------------------------------------
$downloaded = 0
$skipped = 0
$failed = @()

foreach ($hero in $heroes) {
    # Internal name is the stable key everywhere else in this repo (GSI sends
    # npc_dota_hero_*), so name files by it rather than by the CDN basename.
    $slug = $hero.name -replace '^npc_dota_hero_', ''
    $target = Join-Path $PortraitDir "$slug.png"

    if (-not $Force -and (Test-Path $target) -and (Get-Item $target).Length -gt 0) {
        $skipped++
        continue
    }

    if ([string]::IsNullOrWhiteSpace($hero.img)) {
        $failed += "$slug (no img path in catalog)"
        continue
    }

    # The catalog's img ends in a bare '?' cache-buster; harmless but noisy.
    $url = $CdnBase + ($hero.img -replace '\?$', '')

    try {
        Invoke-Download -Uri $url -OutFile $target
        $downloaded++
        Write-Host "  $slug" -ForegroundColor DarkGray
    } catch {
        $failed += "$slug ($($_.Exception.Message))"
        if (Test-Path $target) { Remove-Item $target -Force }
    }
}

Write-Host ""
Write-Host "Portraits: $downloaded downloaded, $skipped already cached, $($failed.Count) failed."
Write-Host "Location:  $((Resolve-Path $PortraitDir).Path)"

if ($failed.Count -gt 0) {
    Write-Host ""
    Write-Host "Failed:" -ForegroundColor Yellow
    $failed | ForEach-Object { Write-Host "  $_" -ForegroundColor Yellow }
    exit 1
}
