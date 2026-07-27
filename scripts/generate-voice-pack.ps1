<#
.SYNOPSIS
    Generate a spoken voice pack for objective alerts.

.DESCRIPTION
    Writes one .wav per alert event into assets/voice/<PackName>/, named after the
    event keys the app looks up (power_rune.wav, wisdom_rune.wav, ...).

    Uses the Windows speech synthesiser, so it needs no API key, no network, and
    no account. The voices are serviceable rather than good; for a better pack,
    generate the same filenames with a hosted TTS service and drop them in the
    same directory. The app only cares about the filenames.

    Packs are deliberately not committed to the repository — they are generated
    output, and binary assets in git age badly.

.PARAMETER PackName
    Directory name under assets/voice/. Appears in the app's voice pack picker.

.PARAMETER Voice
    Installed voice name. Omit to use the system default.
    List available voices with:
      Add-Type -AssemblyName System.Speech
      (New-Object System.Speech.Synthesis.SpeechSynthesizer).GetInstalledVoices() |
        ForEach-Object { $_.VoiceInfo.Name }

.PARAMETER Rate
    Speaking rate, -10 (slowest) to 10 (fastest). Callouts want to be brisk:
    the point is to hear them before the objective, not during it.

.EXAMPLE
    ./scripts/generate-voice-pack.ps1
    Generates assets/voice/en-sapi/ with the default voice.

.EXAMPLE
    ./scripts/generate-voice-pack.ps1 -PackName zira -Voice "Microsoft Zira Desktop"
#>
[CmdletBinding()]
param(
    [string]$PackName = "en-sapi",
    [string]$Voice = "",
    [ValidateRange(-10, 10)]
    [int]$Rate = 2
)

$ErrorActionPreference = "Stop"

# Keep callouts to one or two words. A spoken cue is unambiguous on first
# hearing, which is exactly where it beats a synthesised motif — but only if it
# finishes before the objective it is announcing.
$Callouts = [ordered]@{
    power_rune   = "Power rune"
    wisdom_rune  = "Wisdom rune"
    water_rune   = "Water rune"
    bounty_rune  = "Bounty"
    tormentor    = "Tormentor"
    neutral_item = "Neutrals"
    stack        = "Stack"
}

$repoRoot = Split-Path -Parent $PSScriptRoot
$outputDir = Join-Path $repoRoot "assets/voice/$PackName"

if (-not (Test-Path $outputDir)) {
    New-Item -ItemType Directory -Path $outputDir -Force | Out-Null
}

Add-Type -AssemblyName System.Speech
$synth = New-Object System.Speech.Synthesis.SpeechSynthesizer

try {
    if ($Voice) {
        $synth.SelectVoice($Voice)
    }
    $synth.Rate = $Rate

    foreach ($entry in $Callouts.GetEnumerator()) {
        $path = Join-Path $outputDir "$($entry.Key).wav"
        $synth.SetOutputToWaveFile($path)
        $synth.Speak($entry.Value)
        Write-Host "  $($entry.Key).wav  -  `"$($entry.Value)`""
    }
}
finally {
    # Release the last wave file handle before the script exits.
    $synth.SetOutputToNull()
    $synth.Dispose()
}

Write-Host ""
Write-Host "Voice pack '$PackName' written to $outputDir"
Write-Host "Select it on the Alerts page, or set voice_pack = `"$PackName`" under [alerts]."
