<#
.SYNOPSIS
    Pulls a Dota patch's notes, measures hero win rates around it, and scores which
    heroes could go meta. Output lands in docs/patch-analysis/<patch>/.

.DESCRIPTION
    Three steps, run all by default or pick with switches:

      -Notes    Valve's patch feed -> notes.md, the full notes with hero, item and
                ability ids resolved to names.
      -Measure  OpenDota public_matches (All Pick) -> per-hero picks/wins for a
                window before the release and one after it, all ranks and Divine+.
      -Score    Baseline strength from the "before" window plus the hand-written
                patch rating in deltas.csv -> scored.csv. When the "after" window has
                data, its win rates and the change are added alongside.

    Valve's patch_timestamp is a date stamp, not the moment the patch went live: 7.40
    was stamped 08:00 UTC and reached matchmaking about 17 hours later. Pass
    -ReleaseUtc once the real release time is known, or the "after" window will be
    mostly old-patch games and every hero will look unchanged.

    deltas.csv is judgment, not data: one row per hero the patch moved, rated -3
    (big nerf) to +3 (big buff), with the reason. Write it after reading notes.md.

.PARAMETER Patch
    Patch number as Valve writes it, e.g. 7.42 or 7.42b.

.PARAMETER ReleaseUtc
    When the patch went live. Splits the before/after windows. Defaults to Valve's stamp.

.PARAMETER BeforeHours
    Length of the baseline window that ends at -ReleaseUtc. Default 36.

.PARAMETER AfterHours
    Length of the post-release window. 0 (default) means up to now.

.PARAMETER MinAfterHours
    Skip the "after" window until this many hours of post-release games exist. Default 24.

.PARAMETER MinRankTier
    avg_rank_tier floor for the high-rank split. 70 = Divine 0.

.PARAMETER ChunkHours
    OpenDota's explorer times out on long windows; queries are split into chunks of
    this many hours and summed. Default 12.

.EXAMPLE
    pwsh scripts/patch-analysis.ps1 -Patch 7.42

.EXAMPLE
    pwsh scripts/patch-analysis.ps1 -Patch 7.42 -Measure -Score -ReleaseUtc 2026-11-19T00:50Z -BeforeHours 72
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$Patch,
    [switch]$Notes,
    [switch]$Measure,
    [switch]$Score,
    [Nullable[datetime]]$ReleaseUtc,
    [int]$BeforeHours = 36,
    [int]$AfterHours = 0,
    [int]$MinAfterHours = 24,
    [int]$MinRankTier = 70,
    [int]$ChunkHours = 12
)

$ErrorActionPreference = 'Stop'
# CSVs and Markdown must not pick up a decimal comma from the machine's locale.
[cultureinfo]::CurrentCulture = [cultureinfo]::InvariantCulture

if (-not ($Notes -or $Measure -or $Score)) { $Notes = $Measure = $Score = $true }

$RepoRoot = Split-Path $PSScriptRoot -Parent
$OutDir   = Join-Path $RepoRoot "docs\patch-analysis\$Patch"
$DataDir  = Join-Path $OutDir 'data'
$CacheDir = Join-Path $RepoRoot ".cache\patch-analysis\$Patch"
foreach ($dir in $OutDir, $DataDir, $CacheDir) { New-Item -ItemType Directory -Path $dir -Force | Out-Null }

$Feed = 'https://www.dota2.com/datafeed'
$Explorer = 'https://api.opendota.com/api/explorer'

# Hero entries in the patch feed that the hero list does not carry.
$ExtraHeroes = @{ 1961 = 'Spirit Bear' }

function Get-Cached {
    param([string]$Name, [string]$Url)
    $path = Join-Path $CacheDir "$Name.json"
    if (-not (Test-Path $path)) {
        Write-Host "fetch   $Name.json"
        Invoke-WebRequest -Uri $Url -OutFile $path -UseBasicParsing
    }
    Get-Content $path -Raw | ConvertFrom-Json
}

function Get-HeroNames {
    $names = @{}
    (Get-Cached 'herolist' "$Feed/herolist?language=english").result.data.heroes |
        ForEach-Object { $names[[int]$_.id] = $_.name_english_loc }
    foreach ($id in $ExtraHeroes.Keys) { $names[$id] = $ExtraHeroes[$id] }
    $names
}

function Get-ValveStamp {
    $list = Get-Cached 'patchnoteslist' "$Feed/patchnoteslist?language=english"
    $entry = $list.patches | Where-Object { $_.patch_number -eq $Patch }
    if (-not $entry) { throw "Patch $Patch is not in Valve's patch list." }
    [DateTimeOffset]::FromUnixTimeSeconds($entry.patch_timestamp).UtcDateTime
}

# --- Notes -------------------------------------------------------------------------

function ConvertTo-PlainText([string]$Text) {
    if (-not $Text) { return '' }
    ($Text -replace '<br>', ' | ' -replace '<[^>]+>', '').Trim()
}

function Format-NoteLines($List, $Names) {
    foreach ($n in $List) {
        $text = ConvertTo-PlainText $n.note
        if (-not $text -or $text -eq '|') { continue }
        $tag = if ($n.aghanims) { "[$($n.aghanims)] " } else { '' }
        $line = ('  ' * [int]$n.indent_level) + "- $tag$text"
        if ($n.info) { $line += '  (' + (ConvertTo-PlainText $n.info) + ')' }
        $line
    }
}

function Format-ItemSection($Entries, $Names) {
    foreach ($e in $Entries) {
        $name = if ($e.ability_id -gt 0) { $Names[[int]$e.ability_id] } else { $null }
        if ($e.title) { "### " + (ConvertTo-PlainText $e.title) + $(if ($name) { ": $name" } else { '' }) }
        elseif ($name) { "#### $name" }
        Format-NoteLines $e.ability_notes
    }
}

if ($Notes) {
    $stamp = Get-ValveStamp
    $p = Get-Cached 'patchnotes' "$Feed/patchnotes?version=$Patch&language=english"
    $heroes = Get-HeroNames
    $abil = @{}
    foreach ($src in @(
            (Get-Cached 'abilitylist' "$Feed/abilitylist?language=english"),
            (Get-Cached 'itemlist' "$Feed/itemlist?language=english"))) {
        $src.result.data.itemabilities | ForEach-Object { $abil[[int]$_.id] = $_.name_english_loc }
    }

    $md = & {
        "# Patch $Patch notes"
        ''
        "Generated by ``scripts/patch-analysis.ps1`` from Valve's patch feed. Valve stamp: $($stamp.ToString('yyyy-MM-dd HH:mm')) UTC."
        ''
        '# General'
        foreach ($g in $p.general_notes) { "## $($g.title)"; Format-NoteLines $g.generic }
        '# Items'; Format-ItemSection $p.items $abil
        '# Neutral items'; Format-ItemSection $p.neutral_items $abil
        if ($p.neutral_creeps) { '# Neutral creeps'; Format-ItemSection $p.neutral_creeps $abil }
        '# Heroes'
        foreach ($h in $p.heroes) {
            $name = $heroes[[int]$h.hero_id]
            '## ' + $(if ($name) { $name } else { "Hero id $($h.hero_id)" })
            Format-NoteLines $h.hero_notes
            foreach ($s in $h.subsections) { "### $($s.title)"; Format-NoteLines $s.general_notes }
            foreach ($ab in $h.abilities) {
                $an = $abil[[int]$ab.ability_id]
                '### ' + $(if ($an) { $an } else { "Ability id $($ab.ability_id)" })
                Format-NoteLines $ab.ability_notes
            }
            if ($h.talent_notes) { '### Talents'; Format-NoteLines $h.talent_notes }
        }
    }
    $md | Set-Content (Join-Path $OutDir 'notes.md') -Encoding utf8
    Write-Host "wrote   notes.md ($($p.heroes.Count) heroes, $($p.items.Count) item entries, $($p.neutral_items.Count) neutral entries)"
}

# --- Measure -----------------------------------------------------------------------

function Invoke-HeroQuery([long]$From, [long]$To, [int]$RankFloor) {
    $rank = if ($RankFloor -gt 0) { " AND avg_rank_tier >= $RankFloor" } else { '' }
    $where = "start_time >= $From AND start_time < $To AND game_mode = 22$rank"
    $sql = @"
SELECT hero, count(*) picks, sum(win) wins FROM (
 SELECT unnest(radiant_team) hero, radiant_win::int win FROM public_matches WHERE $where
 UNION ALL
 SELECT unnest(dire_team) hero, (NOT radiant_win)::int win FROM public_matches WHERE $where
) t GROUP BY hero
"@
    $r = Invoke-RestMethod -Uri "$Explorer`?sql=$([uri]::EscapeDataString($sql))" -TimeoutSec 300
    if ($r.err) { throw "OpenDota explorer: $($r.err)" }
    $r.rows
}

function Measure-Window([string]$Label, [datetime]$From, [datetime]$To, $Names) {
    foreach ($split in @(@{ Name = 'all'; Floor = 0 }, @{ Name = 'high'; Floor = $MinRankTier })) {
        $sum = @{}
        $cursor = $From
        while ($cursor -lt $To) {
            $next = $cursor.AddHours($ChunkHours); if ($next -gt $To) { $next = $To }
            $f = ([DateTimeOffset]$cursor).ToUnixTimeSeconds(); $t = ([DateTimeOffset]$next).ToUnixTimeSeconds()
            foreach ($row in Invoke-HeroQuery $f $t $split.Floor) {
                $id = [int]$row.hero
                if (-not $sum[$id]) { $sum[$id] = @{ p = 0L; w = 0L } }
                $sum[$id].p += [long]$row.picks; $sum[$id].w += [long]$row.wins
            }
            $cursor = $next
        }
        $file = Join-Path $DataDir "$Label-$($split.Name).csv"
        $rows = foreach ($id in ($sum.Keys | Sort-Object)) {
            if (-not $Names[$id]) { continue }
            [pscustomobject]@{ hero_id = $id; hero = $Names[$id]; picks = $sum[$id].p; wins = $sum[$id].w }
        }
        $matchCount = [long][math]::Round((($rows | Measure-Object picks -Sum).Sum) / 10)
        if ($rows) { $rows | Export-Csv $file -NoTypeInformation } else { Remove-Item $file -ErrorAction SilentlyContinue }
        Write-Host ("measure {0,-6} {1,-4} {2:u} -> {3:u}  {4} matches" -f $Label, $split.Name, $From, $To, $matchCount)
        [pscustomobject]@{ window = $Label; split = $split.Name; from_utc = $From.ToString('u'); to_utc = $To.ToString('u'); matches = $matchCount }
    }
}

if ($Measure) {
    $release = if ($ReleaseUtc) { ([datetime]$ReleaseUtc).ToUniversalTime() } else { Get-ValveStamp }
    $now = [datetime]::UtcNow
    $afterEnd = if ($AfterHours -gt 0) { $release.AddHours($AfterHours) } else { $now }
    if ($afterEnd -gt $now) { $afterEnd = $now }
    $names = Get-HeroNames

    $log = @(Measure-Window 'before' $release.AddHours(-$BeforeHours) $release $names)
    $afterSpan = ($afterEnd - $release).TotalHours
    if ($afterSpan -ge $MinAfterHours) { $log += Measure-Window 'after' $release $afterEnd $names }
    else {
        # A few hours of games is noise, and stale files would leak into -Score.
        Remove-Item (Join-Path $DataDir 'after-*.csv') -ErrorAction SilentlyContinue
        Write-Host ("measure after: only {0:N1}h since release (need {1}h), skipped" -f [math]::Max($afterSpan, 0), $MinAfterHours)
    }

    [pscustomobject]@{
        patch = $Patch; release_utc = $release.ToString('u'); release_source = $(if ($ReleaseUtc) { 'argument' } else { 'valve stamp' })
        min_rank_tier = $MinRankTier; game_mode = 'All Pick (22)'; measured_utc = $now.ToString('u'); windows = $log
    } | ConvertTo-Json -Depth 4 | Set-Content (Join-Path $DataDir 'windows.json') -Encoding utf8
}

# --- Score -------------------------------------------------------------------------

function Read-Window([string]$Name) {
    $file = Join-Path $DataDir "$Name.csv"
    $map = @{}
    if (Test-Path $file) { Import-Csv $file | ForEach-Object { $map[$_.hero] = @{ p = [double]$_.picks; w = [double]$_.wins } } }
    $map
}

if ($Score) {
    $deltaFile = Join-Path $OutDir 'deltas.csv'
    if (-not (Test-Path $deltaFile)) { throw "Missing $deltaFile. Write it from notes.md: hero,delta,reason (-3..+3)." }
    $delta = @{}; Import-Csv $deltaFile | ForEach-Object { $delta[$_.hero] = [double]$_.delta }

    $ba = Read-Window 'before-all'; $bh = Read-Window 'before-high'
    $aa = Read-Window 'after-all';  $ah = Read-Window 'after-high'
    if (-not $ba.Count) { throw 'No before-all.csv. Run -Measure first.' }
    foreach ($h in $delta.Keys) { if (-not $ba[$h]) { Write-Warning "deltas.csv names '$h', which has no match data (typo?)" } }

    $total = ($ba.Values | ForEach-Object { $_.p } | Measure-Object -Sum).Sum / 10
    $round = { param($v) if ($null -eq $v) { $null } else { [math]::Round($v, 1) } }

    $rows = foreach ($hero in $ba.Keys) {
        $wr = 100 * $ba[$hero].w / $ba[$hero].p
        $hwr = if ($bh[$hero]) { 100 * $bh[$hero].w / $bh[$hero].p } else { $wr }
        # Baseline: Divine+ weighted 60/40 over all ranks, centred on 50.
        $base = 0.6 * ($hwr - 50) + 0.4 * ($wr - 50)
        $d = if ($delta.ContainsKey($hero)) { $delta[$hero] } else { 0 }
        $after = if ($aa[$hero]) { 100 * $aa[$hero].w / $aa[$hero].p } else { $null }
        $afterHigh = if ($ah[$hero]) { 100 * $ah[$hero].w / $ah[$hero].p } else { $null }
        $z = $null
        if ($null -ne $after) {
            $p1 = $wr / 100; $p2 = $after / 100
            $z = ($p2 - $p1) / [math]::Sqrt($p1 * (1 - $p1) / $ba[$hero].p + $p2 * (1 - $p2) / $aa[$hero].p)
        }
        [pscustomobject]@{
            hero = $hero; wr = (& $round $wr); high_wr = (& $round $hwr); pick_pct = (& $round (100 * $ba[$hero].p / $total))
            base = [math]::Round($base, 2); patch = $d; score = [math]::Round($base + 1.2 * $d, 2)
            after_wr = (& $round $after); after_high_wr = (& $round $afterHigh)
            d_wr = $(if ($null -ne $after) { [math]::Round($after - $wr, 1) }); z = (& $round $z)
        }
    }
    $rows = $rows | Sort-Object score -Descending
    $rows | Export-Csv (Join-Path $OutDir 'scored.csv') -NoTypeInformation
    Write-Host "wrote   scored.csv ($($rows.Count) heroes)"
    $rows | Select-Object -First 25 | Format-Table -AutoSize | Out-Host
}
