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
& pnpm exec tauri @TauriArgs
exit $LASTEXITCODE
