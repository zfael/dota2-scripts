<#
.SYNOPSIS
    Regenerates src/actions/hero_attributes.rs from the OpenDota `dotaconstants` catalog.

.DESCRIPTION
    Power Treads automation needs each hero's primary attribute: treads bought on a hero
    start on it, and it is the attribute the automation returns to after a cast. GSI
    reports the hero's name but never its attribute, so the mapping is baked into a
    generated lookup table.

    Source of truth is https://github.com/odota/dotaconstants, which is itself generated
    from Valve's game files:

      heroes.json   hero id -> `name` (GSI `hero.name`), `primary_attr` (str/agi/int/all)

    Run after a patch that adds a hero or reworks one's attribute. Review the diff.

.PARAMETER Offline
    Reuse previously downloaded JSON in -CacheDir instead of fetching.

.EXAMPLE
    pwsh scripts/generate-hero-attributes.ps1
#>
[CmdletBinding()]
param(
    [string]$OutFile  = (Join-Path $PSScriptRoot "..\src\actions\hero_attributes.rs"),
    [string]$CacheDir = (Join-Path $PSScriptRoot "..\.cache\dotaconstants"),
    [switch]$Offline
)

$ErrorActionPreference = 'Stop'

$BaseUrl = "https://raw.githubusercontent.com/odota/dotaconstants/master/build"

if (-not (Test-Path $CacheDir)) { New-Item -ItemType Directory -Path $CacheDir -Force | Out-Null }

$path = Join-Path $CacheDir "heroes.json"
if ($Offline) {
    if (-not (Test-Path $path)) { throw "-Offline set but $path is missing. Run once without -Offline." }
    Write-Host "cached  heroes.json"
} else {
    Write-Host "fetch   heroes.json"
    Invoke-WebRequest -Uri "$BaseUrl/heroes.json" -OutFile $path -UseBasicParsing
}

$heroes = Get-Content $path -Raw | ConvertFrom-Json

$variants = @{
    'str' = 'Strength'
    'agi' = 'Agility'
    'int' = 'Intelligence'
    'all' = 'Universal'
}

function Format-RustString { param([string]$Text) return '"' + ($Text -replace '\\', '\\' -replace '"', '\"') + '"' }

$rows = [System.Collections.Generic.List[object]]::new()
foreach ($prop in $heroes.PSObject.Properties) {
    $hero = $prop.Value
    $variant = $variants[[string]$hero.primary_attr]
    if (-not $variant) { throw "unknown primary_attr '$($hero.primary_attr)' on $($hero.name) - upstream schema change?" }
    $rows.Add([pscustomobject]@{
        Name    = [string]$hero.name
        Variant = $variant
        Label   = [string]$hero.localized_name
    })
}
$rows = $rows | Sort-Object Name -Unique

Write-Host ""
Write-Host "heroes    : $($rows.Count)"
$rows | Group-Object Variant | Sort-Object Name | ForEach-Object { Write-Host ("  {0,-13}: {1}" -f $_.Name, $_.Count) }

$sb = [System.Text.StringBuilder]::new()
$null = $sb.AppendLine("//! Primary attribute for every Dota hero.")
$null = $sb.AppendLine("//!")
$null = $sb.AppendLine("//! GENERATED FILE - DO NOT EDIT BY HAND.")
$null = $sb.AppendLine("//! Regenerate with ``pwsh scripts/generate-hero-attributes.ps1`` after a patch that adds")
$null = $sb.AppendLine("//! a hero or changes one's attribute.")
$null = $sb.AppendLine("//!")
$null = $sb.AppendLine("//! Source: https://github.com/odota/dotaconstants (generated from Valve's game files).")
$null = $sb.AppendLine("//!")
$null = $sb.AppendLine("//! GSI reports ``hero.name`` but not the hero's attribute. Power Treads bought on a hero")
$null = $sb.AppendLine("//! start on its primary attribute, so this is what seeds the treads state.")
$null = $sb.AppendLine("//!")
$null = $sb.AppendLine("//! A missing key means ``None`` - a hero this table has never heard of.")
$null = $sb.AppendLine("")
$null = $sb.AppendLine("use std::collections::HashMap;")
$null = $sb.AppendLine("use std::sync::LazyLock;")
$null = $sb.AppendLine("")
$null = $sb.AppendLine("/// A hero's primary attribute. ``Universal`` heroes draw damage from all three and have")
$null = $sb.AppendLine("/// no single attribute for Power Treads to start on.")
$null = $sb.AppendLine("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]")
$null = $sb.AppendLine("pub enum PrimaryAttribute {")
$null = $sb.AppendLine("    Strength,")
$null = $sb.AppendLine("    Agility,")
$null = $sb.AppendLine("    Intelligence,")
$null = $sb.AppendLine("    Universal,")
$null = $sb.AppendLine("}")
$null = $sb.AppendLine("")
$null = $sb.AppendLine("/// Primary attribute keyed by GSI ``hero.name``.")
$null = $sb.AppendLine("#[rustfmt::skip]")
$null = $sb.AppendLine("pub static HERO_PRIMARY_ATTRIBUTE_TABLE: &[(&str, PrimaryAttribute)] = &[")
foreach ($row in $rows) {
    $comment = if ($row.Label) { "  // $($row.Label)" } else { "" }
    $null = $sb.AppendLine("    ($(Format-RustString $row.Name), PrimaryAttribute::$($row.Variant)),$comment")
}
$null = $sb.AppendLine("];")
$null = $sb.AppendLine("")
$null = $sb.AppendLine(@'
static HERO_PRIMARY_ATTRIBUTE: LazyLock<HashMap<&'static str, PrimaryAttribute>> =
    LazyLock::new(|| HERO_PRIMARY_ATTRIBUTE_TABLE.iter().copied().collect());

/// The hero's primary attribute, or `None` when the hero is not in the table.
pub fn primary_attribute(hero_name: &str) -> Option<PrimaryAttribute> {
    HERO_PRIMARY_ATTRIBUTE.get(hero_name).copied()
}
'@)

$dir = Split-Path -Parent $OutFile
if (-not (Test-Path $dir)) { New-Item -ItemType Directory -Path $dir -Force | Out-Null }
[System.IO.File]::WriteAllText((Resolve-Path -LiteralPath $dir | Join-Path -ChildPath (Split-Path -Leaf $OutFile)), $sb.ToString(), (New-Object System.Text.UTF8Encoding($false)))

Write-Host ""
Write-Host "wrote $OutFile"
