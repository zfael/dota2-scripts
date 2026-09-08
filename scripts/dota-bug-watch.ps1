<#
.SYNOPSIS
    Pulls new issues from ValveSoftware/Dota2-Gameplay since the last check and
    ranks them by how much they matter to this repo's automation.

.DESCRIPTION
    Valve's public gameplay tracker is mostly store, compendium, matchmaking and
    behaviour-score traffic. This script drops that, then sorts what is left into:

      ACTIONABLE  - touches a hero we script, an item we automate, or the input
                    layer itself (smart cast, double tap, hotkeys, unit select).
      MECHANIC    - a real ability/item bug, but nothing we drive today.

    A cursor at .cache/dota-bug-watch/cursor.json records the newest issue seen,
    so repeat runs only report what has appeared since. The cursor advances to
    the newest issue *fetched*, not to "now", so an issue filed while the script
    is running is not skipped on the next pass.

.PARAMETER Since
    ISO-8601 timestamp overriding the stored cursor. Does not change the cursor
    unless the run also advances it.

.PARAMETER Days
    On a first run with no cursor, how far back to look. Default 7.

.PARAMETER MaxPages
    Safety stop on pagination. 100 issues per page. Default 10.

.PARAMETER NoAdvance
    Report but leave the cursor where it was. Use for exploratory re-runs.

.PARAMETER Requests
    Also list feature requests and balance opinions, which are hidden by default.

.PARAMETER All
    Skip the noise filter and relevance tiers; dump every new issue.

.PARAMETER Json
    Emit the classified issues as JSON instead of a human-readable report.

.EXAMPLE
    ./scripts/dota-bug-watch.ps1
    ./scripts/dota-bug-watch.ps1 -Since 2026-08-20 -NoAdvance
#>
[CmdletBinding()]
param(
    [string]$Since,
    [int]$Days = 7,
    [int]$MaxPages = 10,
    [switch]$NoAdvance,
    [switch]$All,
    [switch]$Requests,
    [switch]$Json
)

$ErrorActionPreference = 'Stop'

$Repo = 'ValveSoftware/Dota2-Gameplay'
$RepoRoot = Split-Path -Parent $PSScriptRoot
$CursorPath = Join-Path $RepoRoot '.cache/dota-bug-watch/cursor.json'

# --- classification -----------------------------------------------------------
# Noise is matched against the TITLE only. Bodies of genuine hero reports often
# mention "ranked match" or a crash, and matching those would drop real bugs.

$NoiseRe = @(
    'compendium|fantasy|percentil|predictions? trophy'
    'marketplace|steam market|treasure|arcana|bundle|cosmetic|courier|immortal (item|set)'
    'refund|purchase|price|drop( rate)?s?\b|battle ?pass|crownfall|carnival|terrain token'
    'candyworks|duplicat|arcane duplicate'
    'collector.s cache|marketable|tradable|tradeable|inventory|item id|steam ?id'
    'terrain|\brendering\b|texture|water quality'
    '\bban(ned)?\b|behaviou?r score|conduct (summary|system)|communication score'
    'smurf|overwatch|low priority|report(ing|ed)? system'
    'medal|dota ?plus|mmr|rank(ed)? (reset|medal|points)|profile|badge|subscription'
    'voice ?line|chat ?wheel|hero challenge'
    'matchmaking|queue time|dedicated server|custom game|arcade|server (down|issue)'
    'crash|\bfps\b|\bping\b|packet loss|black screen|loading time'
) -join '|'

# Heroes with a script under src/actions/heroes/. Hero name plus a few signature
# abilities, so a report titled only "Sacred Arrow does not..." still lands.
# Kept per-hero rather than one blob so the report can name the hero that hit.
$HeroPatterns = [ordered]@{
    'Broodmother'        = 'broodmother|\bbrood\b|spin web|spiderling|incapacitating bite'
    'Earth Spirit'       = 'earth spirit|boulder smash|rolling boulder|geomagnetic grip|magnetize'
    'Ember Spirit'       = 'ember spirit|\bember\b|searing chains|sleight of fist|flame guard|fire remnant'
    'Huskar'             = 'huskar|inner fire|burning spear|berserker.s blood|life break'
    'Invoker'            = 'invoker|quas|wex|exort|sunstrike|tornado|forge spirit|chaos meteor|deafening blast|\bemp\b'
    'Largo'              = 'largo'
    'Legion Commander'   = 'legion commander|\blegion\b|\bduel\b|overwhelming odds|press the attack|moment of courage'
    'Magnus'             = 'magnus|empower|skewer|reverse polarity|shockwave'
    'Meepo'              = 'meepo|\bpoof\b|earthbind|divided we stand'
    'Mirana'             = 'mirana|sacred arrow|starstorm|moonlight shadow'
    'Morphling'          = 'morphling|\bmorph\b|waveform|adaptive strike|attribute shift|\breplicate\b'
    'Outworld Destroyer' = 'outworld destroyer|outworld|astral imprisonment|arcane orb|sanity.s eclipse'
    'Shadow Fiend'       = 'shadow fiend|\bsf\b|shadowraze|necromastery|requiem of souls|presence of the dark lord'
    'Slark'              = 'slark|dark pact|pounce|essence shift|shadow dance'
    'Snapfire'           = 'snapfire|scatterblast|firesnap cookie|lil.? shredder|mortimer'
    'Tiny'               = '\btiny\b|avalanche|\btoss\b|tree grab|\bgrow\b'
}

# Wishlist traffic. Real defects get reported as defects; these are design
# opinions and swamp the bug signal if left in the default report.
$RequestRe = @(
    'feature request|\[suggestion\]|^suggestion\b|suggestion:'
    '\bqol\b|quality of life|please add|\bwould be (nice|great|cool)\b'
    '\brework\b|\bbuff\b|\bnerf\b|overtuned|underwhelming|concept\b|proposal'
) -join '|'

# Items src/actions/ actually casts or reacts to (auto_items, item_automation,
# dispel, defensive_windows, invisibility, armlet, soul_ring).
$ItemRe = @(
    'lotus orb|glimmer cape|\bmanta\b|silver edge|invisibility sword|shadow blade'
    'orchid|magic wand|magic stick|arcane boots|phase boots|ghost scepter|ghost sceptre'
    '\barmlet\b|soul ring|black king bar|\bbkb\b'
    'ash legion shield|crippling crossbow|essence ring|flayer.s bota|idol of screeauk'
    'jidi pollen bag|kobold cup|mana draught|metamorphic mandible|minotaur horn'
    'pogo stick|polliwog charm|psychic headband|riftshadow prism'
) -join '|'

# The input/client layer this project drives with synthetic keystrokes. A bug
# here changes what our own key presses do, so it outranks a hero-specific bug.
$InputRe = @(
    'smart ?cast|quick ?cast|double ?tap|self.?cast|auto.?cast'
    'hotkey|key ?bind|keybind|rebind'
    'unit select|control group|select all|alt.?click|alt ?\+'
    'command queue|shift.?queue|order queue|animation cancel|cast point|turn rate'
    'auto.?attack|auto.?repeat|toggle'
) -join '|'

# Anything left that still looks like a genuine gameplay defect. Deliberately
# free of generic complaint words ("bug", "broken", "incorrect", "wrong",
# "damage") -- those match every compendium and account post on the tracker.
# Every term here has to be something you can only say about game mechanics.
$MechRe = @(
    'dispel|stun|silence|\broot(ed|s)?\b|spell immun|debuff immun|status resist'
    'aghanim|\bshard\b|scepter|sceptre|innate|facet|talent|\bultimate\b'
    'illusion|invisib|\binvis\b|true sight|\bward\b|creep|roshan|tormentor|neutral item'
    'cooldown|\bcast\b|channel|\bproc\b|\bstack(s|ing)?\b|pathing|linken|teleport'
    'tooltip|passive|abilit|attack speed|armou?r|movement speed|spell damage'
    'lifesteal|mana ?cost|magic resist|\bradius\b|\bduration\b|damage type'
) -join '|'

# --- helpers ------------------------------------------------------------------

function ConvertTo-Utc {
    param($Value)
    if ($null -eq $Value) { return $null }
    if ($Value -is [datetime]) { return $Value.ToUniversalTime() }
    return [datetime]::Parse(
        [string]$Value,
        [Globalization.CultureInfo]::InvariantCulture,
        [Globalization.DateTimeStyles]::AdjustToUniversal -bor [Globalization.DateTimeStyles]::AssumeUniversal)
}

function Get-Excerpt {
    param([string]$Body, [int]$Length = 320)
    if ([string]::IsNullOrWhiteSpace($Body)) { return '(no body)' }
    $t = $Body -replace '<img[^>]*>', '' -replace '!?\[[^\]]*\]\([^)]*\)', '' `
               -replace '### (Description|Ability name|Item name|Hero and spell)', '' `
               -replace '_No response_', '' -replace '\r?\n', ' ' -replace '\s+', ' '
    $t = $t.Trim()
    if ($t.Length -eq 0) { return '(no body)' }
    if ($t.Length -gt $Length) { return $t.Substring(0, $Length).TrimEnd() + '...' }
    return $t
}

# --- cursor -------------------------------------------------------------------

$cursor = $null
if (Test-Path $CursorPath) {
    $cursor = Get-Content $CursorPath -Raw | ConvertFrom-Json
}

if ($Since) {
    $sinceUtc = ConvertTo-Utc $Since
    $sinceLabel = "-Since override"
} elseif ($cursor -and $cursor.newest_issue_created_at) {
    $sinceUtc = ConvertTo-Utc $cursor.newest_issue_created_at
    $sinceLabel = "cursor (last checked $($cursor.checked_at))"
} else {
    $sinceUtc = (Get-Date).ToUniversalTime().AddDays(-$Days)
    $sinceLabel = "first run, last $Days days"
}

# --- fetch --------------------------------------------------------------------

$issues = @()
$newestSeen = $null
$reachedCursor = $false

for ($page = 1; $page -le $MaxPages; $page++) {
    $raw = gh api "repos/$Repo/issues?state=all&sort=created&direction=desc&per_page=100&page=$page" 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "gh api failed (is 'gh auth login' done?):`n$raw"
    }
    $batch = $raw | ConvertFrom-Json
    if (-not $batch -or $batch.Count -eq 0) { break }

    foreach ($i in $batch) {
        if ($i.PSObject.Properties.Name -contains 'pull_request' -and $i.pull_request) { continue }
        $created = ConvertTo-Utc $i.created_at
        if ($null -eq $newestSeen -or $created -gt $newestSeen) { $newestSeen = $created }
        if ($created -le $sinceUtc) { $reachedCursor = $true; continue }
        $issues += $i
    }

    if ($reachedCursor) { break }
}

if (-not $reachedCursor -and $issues.Count -gt 0) {
    Write-Warning "Hit the -MaxPages limit ($MaxPages) before reaching the cursor; older new issues may be missing."
}

# --- classify -----------------------------------------------------------------

$classified = foreach ($i in $issues) {
    $title = [string]$i.title
    $text = "$title`n$($i.body)"

    $tags = @()
    if ($text -imatch $InputRe) { $tags += 'input' }
    foreach ($hero in $HeroPatterns.Keys) {
        if ($text -imatch $HeroPatterns[$hero]) { $tags += $hero }
    }
    if ($text -imatch $ItemRe) { $tags += 'item' }
    $hasMech = $text -imatch $MechRe

    $tier = if ($tags.Count -gt 0) { 'ACTIONABLE' } elseif ($hasMech) { 'MECHANIC' } else { 'OTHER' }
    if (-not $All) {
        # A request stays visible under its own tier, but out of the bug report.
        if ($title -imatch $RequestRe) { $tier = 'REQUEST' }
        if ($title -imatch $NoiseRe) { $tier = 'NOISE' }
    }

    [pscustomobject]@{
        number     = $i.number
        title      = $title
        url        = $i.html_url
        created_at = (ConvertTo-Utc $i.created_at).ToString('yyyy-MM-ddTHH:mm:ssZ')
        state      = $i.state
        comments   = $i.comments
        reactions  = $i.reactions.total_count
        tier       = $tier
        tags       = $tags
        excerpt    = (Get-Excerpt $i.body)
    }
}

$wanted = if ($Requests) { @('ACTIONABLE', 'MECHANIC', 'REQUEST') } else { @('ACTIONABLE', 'MECHANIC') }
$keep = if ($All) { $classified } else { $classified | Where-Object { $_.tier -in $wanted } }
$keep = @($keep | Sort-Object @{ E = { $_.tier } }, @{ E = { -($_.comments + $_.reactions) } }, @{ E = { $_.number }; Descending = $true })

# --- advance cursor -----------------------------------------------------------

if (-not $NoAdvance -and $newestSeen) {
    $dir = Split-Path -Parent $CursorPath
    if (-not (Test-Path $dir)) { New-Item -ItemType Directory -Force -Path $dir | Out-Null }
    [pscustomobject]@{
        newest_issue_created_at = $newestSeen.ToString('yyyy-MM-ddTHH:mm:ssZ')
        checked_at              = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
        previous_cursor         = if ($cursor) { $cursor.newest_issue_created_at } else { $null }
        issues_seen             = $issues.Count
    } | ConvertTo-Json | Set-Content $CursorPath -Encoding utf8
}

# --- report -------------------------------------------------------------------

if ($Json) {
    $keep | ConvertTo-Json -Depth 4
    return
}

$since = $sinceUtc.ToString('yyyy-MM-dd HH:mm') + 'Z'
Write-Host ""
Write-Host "Dota2-Gameplay issues since $since  [$sinceLabel]"
Write-Host ("  {0} new, {1} after filtering{2}" -f $issues.Count, $keep.Count, $(if ($NoAdvance) { ', cursor NOT advanced' } else { '' }))
Write-Host ""

if ($keep.Count -eq 0) {
    Write-Host "  Nothing new worth reading."
    return
}

foreach ($group in 'ACTIONABLE', 'MECHANIC', 'REQUEST', 'OTHER', 'NOISE') {
    $rows = @($keep | Where-Object { $_.tier -eq $group })
    if ($rows.Count -eq 0) { continue }
    Write-Host "== $group ($($rows.Count))"
    Write-Host ""
    foreach ($r in $rows) {
        $tag = if ($r.tags.Count) { ' {' + ($r.tags -join ',') + '}' } else { '' }
        $st = if ($r.state -ne 'open') { " [$($r.state)]" } else { '' }
        Write-Host ("  #{0} {1}{2}{3}  [{4}c/{5}r]" -f $r.number, $r.created_at.Substring(0, 10), $st, $tag, $r.comments, $r.reactions)
        Write-Host ("     {0}" -f $r.title)
        Write-Host ("     {0}" -f $r.excerpt)
        Write-Host ("     {0}" -f $r.url)
        Write-Host ""
    }
}
