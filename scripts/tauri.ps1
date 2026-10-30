param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]] $TauriArgs
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path

function Test-IsAdministrator {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = [Security.Principal.WindowsPrincipal]::new($identity)
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

$isDevCommand = $TauriArgs.Count -gt 0 -and $TauriArgs[0] -eq "dev"
if ($isDevCommand -and -not (Test-IsAdministrator)) {
    $shell = (Get-Process -Id $PID).Path
    $scriptArguments = @(
        "-NoLogo"
        "-NoProfile"
        "-ExecutionPolicy"
        "Bypass"
        "-File"
        ('"{0}"' -f $PSCommandPath)
    ) + $TauriArgs
    $process = Start-Process `
        -FilePath $shell `
        -ArgumentList $scriptArguments `
        -WorkingDirectory $repoRoot `
        -Verb RunAs `
        -Wait `
        -PassThru
    exit $process.ExitCode
}

Set-Location -LiteralPath $repoRoot
$isBuild = $TauriArgs.Count -gt 0 -and $TauriArgs[0] -eq "build" -and
    -not ($TauriArgs | Where-Object { $_ -cin @('--help', '-h', '--version', '-V') })
$previousKey = $env:TAURI_SIGNING_PRIVATE_KEY
try {
    if ($isBuild -and -not $previousKey) {
        $keyPath = Join-Path $env:LOCALAPPDATA "IdentityV-Assistant/release-keys/updater.key"
        if (-not (Test-Path -LiteralPath $keyPath -PathType Leaf)) {
            throw "Restore the original updater signing key to $keyPath or set TAURI_SIGNING_PRIVATE_KEY. Never replace the signing key of an existing release. See docs/updates.md."
        }
        $env:TAURI_SIGNING_PRIVATE_KEY = $keyPath
    }
    & pnpm exec tauri @TauriArgs
    $buildExit = $LASTEXITCODE
    if ($buildExit -eq 0 -and $isBuild -and $TauriArgs -notcontains "--debug" -and $TauriArgs -notcontains "--no-bundle") {
        & (Join-Path $PSScriptRoot "update_manifest.ps1")
    }
    exit $buildExit
}
finally {
    $env:TAURI_SIGNING_PRIVATE_KEY = $previousKey
}
