<#
.SYNOPSIS
    Regenerates src/models/neutral_catalog.rs from Valve's Dota datafeed.

.DESCRIPTION
    The Survivability page lists every neutral item in the current pool, by tier, so the
    allowlist is a checklist instead of free text. Valve reshuffles the pool most patches,
    so the list is generated rather than hand-maintained.

    Reads the same feed and cache as generate-mana-costs.ps1:

      datafeed/itemlist                every item id, with `neutral_item_tier` (0-based; -1 = not a neutral)
      datafeed/itemdata?item_id=       an item -> `behavior` bit flags (passive, no target, unit, point)

    Run after a Dota gameplay patch, alongside generate-mana-costs.ps1.

.PARAMETER Offline
    Reuse previously downloaded JSON in -CacheDir instead of fetching.

.EXAMPLE
    pwsh scripts/generate-neutral-catalog.ps1
#>
[CmdletBinding()]
param(
    [string]$OutFile  = (Join-Path $PSScriptRoot "..\src\models\neutral_catalog.rs"),
    [string]$CacheDir = (Join-Path $PSScriptRoot "..\.cache\valve-datafeed"),
    [switch]$Offline
)

$ErrorActionPreference = 'Stop'
$BaseUrl = "https://www.dota2.com/datafeed"

if (-not (Test-Path $CacheDir)) { New-Item -ItemType Directory -Path $CacheDir -Force | Out-Null }

function Read-CachedJson {
    param([string]$Path)

    if (-not (Test-Path $Path)) { throw "-Offline set but $Path is missing. Run once without -Offline." }
    return Get-Content $Path -Raw | ConvertFrom-Json
}

# `DOTA_ABILITY_BEHAVIOR_*` bits, most specific first: an item that takes a point or a
# unit is cast that way even when it also carries the no-target bit.
function ConvertTo-Targeting {
    param([UInt64]$Behavior)

    if ($Behavior -band 16) { return 'PointTarget' }
    if ($Behavior -band 8)  { return 'UnitTarget' }
    if ($Behavior -band 4)  { return 'NoTarget' }
    return 'Passive'
}

function Format-RustString { param([string]$Text) return '"' + ($Text -replace '\\', '\\' -replace '"', '\"') + '"' }

$listPath = Join-Path $CacheDir "itemlist.json"
if (-not $Offline) {
    Invoke-WebRequest -Uri "$BaseUrl/itemlist?language=english" -OutFile $listPath -UseBasicParsing
}
$neutrals = @((Read-CachedJson $listPath).result.data.itemabilities |
    Where-Object { $_.neutral_item_tier -ge 0 -and $_.name -like 'item_*' })

$itemPath = { param($id) Join-Path $CacheDir "itemdata\$id.json" }
if ($Offline) {
    Write-Host "neutrals  : $($neutrals.Count) (cached)"
} else {
    Write-Host "neutrals  : $($neutrals.Count) (fetching)"
    # Valve's feed takes about a second per request. Needs PowerShell 7.
    $neutrals | ForEach-Object -ThrottleLimit 16 -Parallel {
        $path = Join-Path $using:CacheDir "itemdata\$($_.id).json"
        $dir = Split-Path -Parent $path
        if (-not (Test-Path $dir)) { New-Item -ItemType Directory -Path $dir -Force | Out-Null }
        Invoke-WebRequest -Uri "$using:BaseUrl/itemdata?language=english&item_id=$($_.id)" -OutFile $path -UseBasicParsing
    }
}

$rows = foreach ($item in $neutrals) {
    $data = (Read-CachedJson (& $itemPath $item.id)).result.data.items[0]
    if (-not $data) { Write-Warning "no itemdata for $($item.name)"; continue }
    [pscustomobject]@{
        Name      = $item.name
        Label     = ($item.name_english_loc -replace '\s+', ' ').Trim()
        Tier      = [int]$item.neutral_item_tier + 1
        Targeting = ConvertTo-Targeting ([UInt64]$data.behavior)
    }
}
$rows = @($rows | Sort-Object Tier, Label)

foreach ($tier in ($rows | Group-Object Tier)) {
    Write-Host "tier $($tier.Name)    : $($tier.Count)"
}

$sb = [System.Text.StringBuilder]::new()
$null = $sb.AppendLine("//! The neutral item pool, by tier.")
$null = $sb.AppendLine("//!")
$null = $sb.AppendLine("//! GENERATED FILE - DO NOT EDIT BY HAND.")
$null = $sb.AppendLine("//! Regenerate with ``pwsh scripts/generate-neutral-catalog.ps1`` after a gameplay patch.")
$null = $sb.AppendLine("//!")
$null = $sb.AppendLine("//! Source: Valve's datafeed (https://www.dota2.com/datafeed), the data behind dota2.com.")
$null = $sb.AppendLine("//!")
$null = $sb.AppendLine("//! This only says what is in the pool and how each item is cast. Whether the runtime")
$null = $sb.AppendLine("//! automates one is ``src/actions/item_automation.rs``'s call.")
$null = $sb.AppendLine("")
$null = $sb.AppendLine("/// How the game casts the item, from its ``DOTA_ABILITY_BEHAVIOR`` flags.")
$null = $sb.AppendLine("#[derive(Debug, Clone, Copy, PartialEq, Eq)]")
$null = $sb.AppendLine("pub enum NeutralTargeting {")
$null = $sb.AppendLine("    Passive,")
$null = $sb.AppendLine("    NoTarget,")
$null = $sb.AppendLine("    UnitTarget,")
$null = $sb.AppendLine("    PointTarget,")
$null = $sb.AppendLine("}")
$null = $sb.AppendLine("")
$null = $sb.AppendLine("#[derive(Debug, Clone, Copy, PartialEq, Eq)]")
$null = $sb.AppendLine("pub struct NeutralCatalogEntry {")
$null = $sb.AppendLine("    /// GSI's ``item.name``.")
$null = $sb.AppendLine("    pub name: &'static str,")
$null = $sb.AppendLine("    pub display_name: &'static str,")
$null = $sb.AppendLine("    /// 1-based, as the game shows it.")
$null = $sb.AppendLine("    pub tier: u8,")
$null = $sb.AppendLine("    pub targeting: NeutralTargeting,")
$null = $sb.AppendLine("}")
$null = $sb.AppendLine("")
$null = $sb.AppendLine("#[rustfmt::skip]")
$null = $sb.AppendLine("pub static NEUTRAL_ITEM_CATALOG: &[NeutralCatalogEntry] = &[")
foreach ($row in $rows) {
    $null = $sb.AppendLine("    NeutralCatalogEntry { name: $(Format-RustString $row.Name), display_name: $(Format-RustString $row.Label), tier: $($row.Tier), targeting: NeutralTargeting::$($row.Targeting) },")
}
$null = $sb.AppendLine("];")

$dir = Split-Path -Parent $OutFile
if (-not (Test-Path $dir)) { New-Item -ItemType Directory -Path $dir -Force | Out-Null }
[System.IO.File]::WriteAllText((Resolve-Path -LiteralPath $dir | Join-Path -ChildPath (Split-Path -Leaf $OutFile)), $sb.ToString(), (New-Object System.Text.UTF8Encoding($false)))

Write-Host ""
Write-Host "wrote $OutFile"
