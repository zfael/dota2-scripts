<#
.SYNOPSIS
    Regenerates src/actions/mana_costs.rs from the OpenDota `dotaconstants` catalogs.

.DESCRIPTION
    Soul Ring must only fire ahead of something that actually costs mana. Neither GSI
    nor the Dota console exposes a mana cost, so the costs are baked into a generated
    lookup table instead of hand-maintained skip lists.

    Source of truth is https://github.com/odota/dotaconstants, which is itself generated
    from Valve's game files:

      items.json          item name  -> `mc` (single value)
      abilities.json      ability    -> `mc` (per-level array)
      hero_abilities.json hero       -> ability slot list

    Run after a Dota gameplay patch. Review the diff before committing — a mass change to
    zero usually means an upstream schema change, not a balance patch.

.PARAMETER Offline
    Reuse previously downloaded JSON in -CacheDir instead of fetching.

.EXAMPLE
    pwsh scripts/generate-mana-costs.ps1
#>
[CmdletBinding()]
param(
    [string]$OutFile  = (Join-Path $PSScriptRoot "..\src\actions\mana_costs.rs"),
    [string]$CacheDir = (Join-Path $PSScriptRoot "..\.cache\dotaconstants"),
    [switch]$Offline
)

$ErrorActionPreference = 'Stop'

$BaseUrl = "https://raw.githubusercontent.com/odota/dotaconstants/master/build"
$Sources = @('items', 'abilities', 'hero_abilities')

if (-not (Test-Path $CacheDir)) { New-Item -ItemType Directory -Path $CacheDir -Force | Out-Null }

foreach ($name in $Sources) {
    $path = Join-Path $CacheDir "$name.json"
    if ($Offline) {
        if (-not (Test-Path $path)) { throw "-Offline set but $path is missing. Run once without -Offline." }
        Write-Host "cached  $name.json"
        continue
    }
    Write-Host "fetch   $name.json"
    Invoke-WebRequest -Uri "$BaseUrl/$name.json" -OutFile $path -UseBasicParsing
}

$items        = Get-Content (Join-Path $CacheDir "items.json")          -Raw | ConvertFrom-Json
$abilities    = Get-Content (Join-Path $CacheDir "abilities.json")      -Raw | ConvertFrom-Json
$heroAbil     = Get-Content (Join-Path $CacheDir "hero_abilities.json") -Raw | ConvertFrom-Json

# `mc` is `false` when the entry costs no mana, a bare number/string when the cost is flat,
# and an array when it scales per level. Normalise all three to a u32 list.
function ConvertTo-CostList {
    param($Value)

    if ($null -eq $Value) { return @() }
    if ($Value -is [bool]) { return @(0) }          # `false` == free, which is a real answer

    $out = @()
    foreach ($entry in @($Value)) {
        $parsed = 0.0
        if ([double]::TryParse([string]$entry, [ref]$parsed)) {
            $out += [uint32][math]::Round($parsed)
        } else {
            # Non-numeric costs exist upstream (e.g. "0.5% of max mana"). Treat as unknown-free
            # rather than guessing; the runtime falls back to not triggering.
            $out += [uint32]0
        }
    }
    return $out
}

function Format-RustString { param([string]$Text) return '"' + ($Text -replace '\\', '\\' -replace '"', '\"') + '"' }

# ---------------------------------------------------------------- items

$itemRows = [System.Collections.Generic.List[object]]::new()
foreach ($prop in $items.PSObject.Properties) {
    $costs = ConvertTo-CostList $prop.Value.mc
    $cost  = if ($costs.Count -gt 0) { ($costs | Measure-Object -Maximum).Maximum } else { 0 }
    $itemRows.Add([pscustomobject]@{
        Name  = "item_$($prop.Name)"
        Cost  = [uint32]$cost
        Label = $prop.Value.dname
    })
}
$itemRows = $itemRows | Sort-Object Name -Unique

# ------------------------------------------------------------ abilities

# Only abilities that occupy a hero's slots can sit behind Q/W/E/R/D/F.
$heroAbilityNames = [System.Collections.Generic.HashSet[string]]::new()
foreach ($hero in $heroAbil.PSObject.Properties) {
    foreach ($name in $hero.Value.abilities) { [void]$heroAbilityNames.Add($name) }
}

$abilityRows = [System.Collections.Generic.List[object]]::new()
foreach ($name in $heroAbilityNames) {
    $entry = $abilities.$name
    if (-not $entry) { continue }
    $costs = ConvertTo-CostList $entry.mc
    if ($costs.Count -eq 0) { $costs = @(0) }
    $abilityRows.Add([pscustomobject]@{
        Name  = $name
        Costs = $costs
        Label = $entry.dname
    })
}
$abilityRows = $abilityRows | Sort-Object Name -Unique

Write-Host ""
Write-Host "items     : $($itemRows.Count) ($(($itemRows    | Where-Object { $_.Cost -gt 0 }).Count) cost mana)"
Write-Host "abilities : $($abilityRows.Count) ($(($abilityRows | Where-Object { ($_.Costs | Measure-Object -Maximum).Maximum -gt 0 }).Count) cost mana)"

# --------------------------------------------------------------- emit

$sb = [System.Text.StringBuilder]::new()
$null = $sb.AppendLine("//! Mana costs for every Dota item and hero ability.")
$null = $sb.AppendLine("//!")
$null = $sb.AppendLine("//! GENERATED FILE - DO NOT EDIT BY HAND.")
$null = $sb.AppendLine("//! Regenerate with ``pwsh scripts/generate-mana-costs.ps1`` after a gameplay patch.")
$null = $sb.AppendLine("//!")
$null = $sb.AppendLine("//! Source: https://github.com/odota/dotaconstants (generated from Valve's game files).")
$null = $sb.AppendLine("//!")
$null = $sb.AppendLine("//! Soul Ring trades 170 HP for mana, so it must only fire ahead of something that")
$null = $sb.AppendLine("//! actually spends mana. GSI reports readiness (``can_cast``, ``cooldown``) but never a")
$null = $sb.AppendLine("//! cost, so the cost has to come from here.")
$null = $sb.AppendLine("//!")
$null = $sb.AppendLine("//! A missing key means ``None`` - unknown to this table, not free. Callers treat")
$null = $sb.AppendLine('//! unknown as "do not trigger" so a post-patch item fails safe.')
$null = $sb.AppendLine("")
$null = $sb.AppendLine("use std::collections::HashMap;")
$null = $sb.AppendLine("use std::sync::LazyLock;")
$null = $sb.AppendLine("")

$null = $sb.AppendLine("/// Flat item mana costs, keyed by GSI ``item.name``. ``0`` means the item is known to")
$null = $sb.AppendLine("/// cost no mana (passive, toggle, or a free active such as Quelling Blade's chop).")
$null = $sb.AppendLine("#[rustfmt::skip]")
$null = $sb.AppendLine("pub static ITEM_MANA_COST_TABLE: &[(&str, u32)] = &[")
foreach ($row in $itemRows) {
    $comment = if ($row.Label) { "  // $($row.Label)" } else { "" }
    $null = $sb.AppendLine("    ($(Format-RustString $row.Name), $($row.Cost)),$comment")
}
$null = $sb.AppendLine("];")
$null = $sb.AppendLine("")

$null = $sb.AppendLine("/// Per-level ability mana costs, keyed by GSI ``ability.name``. Index with")
$null = $sb.AppendLine("/// ``ability.level - 1``; see [``ability_mana_cost``] which does the clamping.")
$null = $sb.AppendLine("#[rustfmt::skip]")
$null = $sb.AppendLine("pub static ABILITY_MANA_COST_TABLE: &[(&str, &[u32])] = &[")
foreach ($row in $abilityRows) {
    $joined  = ($row.Costs -join ", ")
    $comment = if ($row.Label) { "  // $($row.Label)" } else { "" }
    $null = $sb.AppendLine("    ($(Format-RustString $row.Name), &[$joined]),$comment")
}
$null = $sb.AppendLine("];")
$null = $sb.AppendLine("")

$null = $sb.AppendLine(@'
static ITEM_MANA_COST: LazyLock<HashMap<&'static str, u32>> =
    LazyLock::new(|| ITEM_MANA_COST_TABLE.iter().copied().collect());

static ABILITY_MANA_COST: LazyLock<HashMap<&'static str, &'static [u32]>> =
    LazyLock::new(|| ABILITY_MANA_COST_TABLE.iter().copied().collect());

/// Mana an item spends when activated, or `None` when the item is not in the table.
///
/// `Some(0)` and `None` mean different things: the first is "known to be free", the
/// second is "this build has never heard of it". Both suppress Soul Ring, but only the
/// second is worth logging.
pub fn item_mana_cost(name: &str) -> Option<u32> {
    ITEM_MANA_COST.get(name).copied()
}

/// Mana an ability spends at `level`, or `None` when the ability is not in the table.
///
/// `level` is GSI's 1-based `ability.level`; level `0` is unlearned and yields `None`.
/// Levels past the end of the table clamp to the last entry, which keeps Aghanim's and
/// talent-granted extra levels from falling off.
pub fn ability_mana_cost(name: &str, level: u32) -> Option<u32> {
    if level == 0 {
        return None;
    }
    let costs = ABILITY_MANA_COST.get(name)?;
    let index = ((level - 1) as usize).min(costs.len().saturating_sub(1));
    costs.get(index).copied()
}
'@)

$dir = Split-Path -Parent $OutFile
if (-not (Test-Path $dir)) { New-Item -ItemType Directory -Path $dir -Force | Out-Null }
[System.IO.File]::WriteAllText((Resolve-Path -LiteralPath $dir | Join-Path -ChildPath (Split-Path -Leaf $OutFile)), $sb.ToString(), (New-Object System.Text.UTF8Encoding($false)))

Write-Host ""
Write-Host "wrote $OutFile"
