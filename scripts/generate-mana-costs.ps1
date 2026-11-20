<#
.SYNOPSIS
    Regenerates src/actions/mana_costs.rs from Valve's Dota datafeed.

.DESCRIPTION
    Soul Ring must only fire ahead of something that actually costs mana. Neither GSI
    nor the Dota console exposes a mana cost, so the costs are baked into a generated
    lookup table instead of hand-maintained skip lists.

    The default source is Valve's own datafeed, the one dota2.com's hero, item and patch
    pages are built from. It is live on patch day:

      datafeed/herolist                every hero id
      datafeed/herodata?hero_id=       a hero's abilities -> `mana_costs` (per-level array)
      datafeed/abilitylist             every ability id
      datafeed/abilitydata?ability_id= one ability -> `mana_costs`
      datafeed/itemlist                every item id
      datafeed/itemdata?item_id=       an item -> `mana_costs`

    `herodata` lists only the abilities dota2.com shows on the hero page, so slot-filling
    sub-abilities (Shadow Fiend's second and third Shadowraze, Puck's Ethereal Jaunt,
    Kunkka's Return) are missing. Every ability named after a hero that `herodata` did
    not cover is fetched from `abilitydata` instead.

    `-Source dotaconstants` reads https://github.com/odota/dotaconstants instead. It is
    generated from the game files too, but can lag a patch by days.

    Run after a Dota gameplay patch. Review the diff before committing - a mass change to
    zero usually means an upstream schema change, not a balance patch.

.PARAMETER Source
    `valve` (default) or `dotaconstants`.

.PARAMETER Offline
    Reuse previously downloaded JSON in -CacheDir instead of fetching.

.EXAMPLE
    pwsh scripts/generate-mana-costs.ps1
#>
[CmdletBinding()]
param(
    [ValidateSet('valve', 'dotaconstants')]
    [string]$Source   = 'valve',
    [string]$OutFile  = (Join-Path $PSScriptRoot "..\src\actions\mana_costs.rs"),
    [string]$CacheDir = (Join-Path $PSScriptRoot "..\.cache\$(if ($Source -eq 'valve') { 'valve-datafeed' } else { 'dotaconstants' })"),
    [switch]$Offline
)

$ErrorActionPreference = 'Stop'

if (-not (Test-Path $CacheDir)) { New-Item -ItemType Directory -Path $CacheDir -Force | Out-Null }

# Downloads $Url to $Path unless -Offline, then returns the parsed JSON.
function Get-CachedJson {
    param([string]$Url, [string]$Path)

    if ($Offline) {
        if (-not (Test-Path $Path)) { throw "-Offline set but $Path is missing. Run once without -Offline." }
    } else {
        $dir = Split-Path -Parent $Path
        if (-not (Test-Path $dir)) { New-Item -ItemType Directory -Path $dir -Force | Out-Null }
        Invoke-WebRequest -Uri $Url -OutFile $Path -UseBasicParsing
    }
    return Get-Content $Path -Raw | ConvertFrom-Json
}

# Downloads many { Url, Path } pairs concurrently; Valve's feed takes about a second per
# request, so one at a time would take ten minutes. Needs PowerShell 7.
function Save-Parallel {
    param([object[]]$Requests, [string]$What)

    if ($Offline) { Write-Host "$($What.PadRight(10)): $($Requests.Count) (cached)"; return }
    Write-Host "$($What.PadRight(10)): $($Requests.Count) (fetching)"
    $Requests | ForEach-Object -ThrottleLimit 16 -Parallel {
        $dir = Split-Path -Parent $_.Path
        if (-not (Test-Path $dir)) { New-Item -ItemType Directory -Path $dir -Force | Out-Null }
        Invoke-WebRequest -Uri $_.Url -OutFile $_.Path -UseBasicParsing
    }
}

function Read-CachedJson {
    param([string]$Path)

    if (-not (Test-Path $Path)) { throw "-Offline set but $Path is missing. Run once without -Offline." }
    return Get-Content $Path -Raw | ConvertFrom-Json
}

# `mc` is `false` when the entry costs no mana, a bare number/string when the cost is flat,
# and an array when it scales per level. Normalise all three to a u32 list.
function ConvertTo-CostList {
    param($Value)

    if ($null -eq $Value) { return @() }
    if ($Value -is [bool]) { return @(0) }          # `false` == free, which is a real answer

    $out = @()
    foreach ($entry in @($Value)) {
        $parsed = 0.0
        if ([double]::TryParse([string]$entry, [System.Globalization.NumberStyles]::Float, [cultureinfo]::InvariantCulture, [ref]$parsed)) {
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

# Display names can carry line breaks ("Black Grimoire\n(Warlock)"), which would end a
# `//` comment early and leave the rest as code.
function Format-Comment { param([string]$Label) if ($Label) { "  // " + ($Label -replace '\s+', ' ').Trim() } else { "" } }

$itemRows    = [System.Collections.Generic.List[object]]::new()
$abilityRows = [System.Collections.Generic.List[object]]::new()

if ($Source -eq 'valve') {
    $BaseUrl = "https://www.dota2.com/datafeed"
    $SourceLine = "Source: Valve's datafeed (https://www.dota2.com/datafeed), the data behind dota2.com."

    # ------------------------------------------------------------ abilities

    $heroes      = (Get-CachedJson "$BaseUrl/herolist?language=english"    (Join-Path $CacheDir "herolist.json")).result.data.heroes
    $abilityList = (Get-CachedJson "$BaseUrl/abilitylist?language=english" (Join-Path $CacheDir "abilitylist.json")).result.data.itemabilities
    # Recipes are scrolls, never cast.
    $itemList    = (Get-CachedJson "$BaseUrl/itemlist?language=english"    (Join-Path $CacheDir "itemlist.json")).result.data.itemabilities |
        Where-Object { $_.name -like 'item_*' -and $_.name -notlike 'item_recipe_*' }

    $heroPath    = { param($id) Join-Path $CacheDir "herodata\$id.json" }
    $abilityPath = { param($id) Join-Path $CacheDir "abilitydata\$id.json" }
    $itemPath    = { param($id) Join-Path $CacheDir "itemdata\$id.json" }

    Save-Parallel -What 'heroes' -Requests @($heroes | ForEach-Object {
        @{ Url = "$BaseUrl/herodata?language=english&hero_id=$($_.id)"; Path = (& $heroPath $_.id) } })
    Save-Parallel -What 'items' -Requests @($itemList | ForEach-Object {
        @{ Url = "$BaseUrl/itemdata?language=english&item_id=$($_.id)"; Path = (& $itemPath $_.id) } })

    # ------------------------------------------------------------ abilities

    $AddAbility = {
        param($Ability)
        $costs = ConvertTo-CostList $Ability.mana_costs
        if ($costs.Count -eq 0) { $costs = @(0) }
        $abilityRows.Add([pscustomobject]@{ Name = $Ability.name; Costs = $costs; Label = $Ability.name_loc })
    }

    $covered = [System.Collections.Generic.HashSet[string]]::new()
    foreach ($hero in $heroes) {
        $data = (Read-CachedJson (& $heroPath $hero.id)).result.data.heroes[0]
        if (-not $data) { Write-Warning "no herodata for $($hero.name)"; continue }

        # Facet-granted abilities sit behind the same slots as the base kit.
        $kit = @($data.abilities) + @($data.facet_abilities | ForEach-Object { $_.abilities })
        foreach ($ability in $kit) {
            if (-not $ability.name -or -not $covered.Add($ability.name)) { continue }
            & $AddAbility $ability
        }
    }

    # Hero abilities are named `<hero>_...` after `npc_dota_hero_<hero>`. Talents are
    # `special_bonus_*` and never match.
    $prefixes = @($heroes | ForEach-Object { ($_.name -replace '^npc_dota_hero_', '') + '_' })
    $extra = @($abilityList | Where-Object {
        $name = $_.name
        -not $covered.Contains($name) -and ($prefixes | Where-Object { $name.StartsWith($_) })
    })
    Save-Parallel -What 'sub-abils' -Requests @($extra | ForEach-Object {
        @{ Url = "$BaseUrl/abilitydata?language=english&ability_id=$($_.id)"; Path = (& $abilityPath $_.id) } })
    foreach ($entry in $extra) {
        $data = (Read-CachedJson (& $abilityPath $entry.id)).result.data.abilities[0]
        if (-not $data) { Write-Warning "no abilitydata for $($entry.name)"; continue }
        & $AddAbility $data
    }

    # ---------------------------------------------------------------- items

    foreach ($item in $itemList) {
        $data = (Read-CachedJson (& $itemPath $item.id)).result.data.items[0]
        if (-not $data) { Write-Warning "no itemdata for $($item.name)"; continue }
        $costs = ConvertTo-CostList $data.mana_costs
        $cost  = if ($costs.Count -gt 0) { ($costs | Measure-Object -Maximum).Maximum } else { 0 }
        $itemRows.Add([pscustomobject]@{ Name = $item.name; Cost = [uint32]$cost; Label = $item.name_english_loc })
    }
} else {
    $BaseUrl = "https://raw.githubusercontent.com/odota/dotaconstants/master/build"
    $SourceLine = "Source: https://github.com/odota/dotaconstants (generated from Valve's game files)."

    $items     = Get-CachedJson "$BaseUrl/items.json"          (Join-Path $CacheDir "items.json")
    $abilities = Get-CachedJson "$BaseUrl/abilities.json"      (Join-Path $CacheDir "abilities.json")
    $heroAbil  = Get-CachedJson "$BaseUrl/hero_abilities.json" (Join-Path $CacheDir "hero_abilities.json")

    foreach ($prop in $items.PSObject.Properties) {
        $costs = ConvertTo-CostList $prop.Value.mc
        $cost  = if ($costs.Count -gt 0) { ($costs | Measure-Object -Maximum).Maximum } else { 0 }
        $itemRows.Add([pscustomobject]@{ Name = "item_$($prop.Name)"; Cost = [uint32]$cost; Label = $prop.Value.dname })
    }

    # Only abilities that occupy a hero's slots can sit behind Q/W/E/R/D/F.
    $heroAbilityNames = [System.Collections.Generic.HashSet[string]]::new()
    foreach ($hero in $heroAbil.PSObject.Properties) {
        foreach ($name in $hero.Value.abilities) { [void]$heroAbilityNames.Add($name) }
    }
    foreach ($name in $heroAbilityNames) {
        $entry = $abilities.$name
        if (-not $entry) { continue }
        $costs = ConvertTo-CostList $entry.mc
        if ($costs.Count -eq 0) { $costs = @(0) }
        $abilityRows.Add([pscustomobject]@{ Name = $name; Costs = $costs; Label = $entry.dname })
    }
}

$itemRows    = $itemRows    | Sort-Object Name -Unique
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
$null = $sb.AppendLine("//! $SourceLine")
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
    $comment = Format-Comment $row.Label
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
    $comment = Format-Comment $row.Label
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
