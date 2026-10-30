$ErrorActionPreference = "Stop"
$repository = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
$originalLocation = Get-Location
$fixture = Join-Path $repository (".tools/release-draft-tests/" + [Guid]::NewGuid().ToString("N"))
$scripts = New-Item -ItemType Directory -Path (Join-Path $fixture "scripts") -Force
$bundle = New-Item -ItemType Directory -Path (Join-Path $fixture "src-tauri/target/release/bundle/nsis") -Force
Copy-Item -LiteralPath (Join-Path $repository "scripts/publish_release.ps1") -Destination $scripts
Copy-Item -LiteralPath (Join-Path $repository "scripts/file_hash.ps1") -Destination $scripts
. (Join-Path $scripts "file_hash.ps1")
function Write-Fixture([string]$path, [string]$text) { [IO.File]::WriteAllText($path, $text, [Text.UTF8Encoding]::new($false)) }
Write-Fixture (Join-Path $fixture "src-tauri/tauri.conf.json") '{"version":"0.1.8"}'
Write-Fixture (Join-Path $fixture "notes.md") "# Fixture`n`n**Full Changelog**: https://example.invalid/compare"
$assetName = "IdentityV-Assistant-0.1.8-Windows-x64-Setup.exe"
$asset = Join-Path $bundle $assetName
Write-Fixture $asset "Offline fixture; never executed or uploaded."
Write-Fixture ($asset + ".sig") "fixture signature"
$digest = Get-ReleaseFileHash $asset
Write-Fixture (Join-Path $bundle "SHA256SUMS.txt") ($digest + "  " + $assetName)
$manifest = @{
    version = "0.1.8"; commit = "fixture-commit"; source_clean = $true; notes = "fixture"; pub_date = "2026-10-30T00:00:00Z"
    platforms = @{ 'windows-x86_64' = @{
        url = "https://github.com/WaltomAdaam2/IdentityV-Assistant/releases/download/v0.1.8/$assetName"
        sha256 = $digest; size = (Get-Item -LiteralPath $asset).Length; signature = "fixture signature"
    } }
}
Write-Fixture (Join-Path $bundle "latest.json") ($manifest | ConvertTo-Json -Depth 5)

# Mock the process boundary. No GitHub request, tag mutation or upload can run.
function git {
    $global:LASTEXITCODE = 0
    switch ($args[0]) {
        "branch" { if ($global:idvDraftTestScenario -eq "wrong-branch") { "main" } else { "SNAPSHOT-v0.1.8" } }
        "status" { if ($global:idvDraftTestScenario -eq "dirty") { " M source.rs" } }
        "rev-parse" { "fixture-commit" }
        default { throw "Unexpected git call: $args" }
    }
}
function gh {
    $global:LASTEXITCODE = 0
    $global:idvDraftTestCalls.Add(($args -join " "))
    if ($args[0] -eq "api") {
        $endpoint = $args | Where-Object { $_ -like "repos/*" } | Select-Object -First 1
        if ($args -contains "POST") {
            $payload = Get-Content -LiteralPath $args[([Array]::IndexOf($args, "--input") + 1)] -Raw | ConvertFrom-Json
            if (-not $payload.draft -or $payload.make_latest -ne "false" -or $payload.target_commitish -ne "fixture-commit") { throw "Unsafe draft payload" }
            "https://example.invalid/draft"
        } elseif ($endpoint -like "*/pulls?*") {
            if ($global:idvDraftTestScenario -eq "frozen") { "1" }
        } elseif ($endpoint -like "*/git/ref/heads/*") {
            if ($global:idvDraftTestScenario -eq "unpushed") { "other-commit" } else { "fixture-commit" }
        } elseif ($endpoint -like "*/releases?*") {
            if ($global:idvDraftTestScenario -eq "existing") { "v0.1.8" }
        } elseif ($endpoint -notlike "*/git/matching-refs/tags/*") { throw "Unexpected API endpoint: $endpoint" }
    } elseif ($args[0] -eq "release" -and $args[1] -eq "upload") {
        if ($args -contains "--clobber") { throw "Must not replace assets" }
    } elseif ($args[0] -eq "release" -and $args[1] -eq "view") {
        '{"url":"https://example.invalid/draft","isDraft":true,"targetCommitish":"fixture-commit","assets":[{},{},{},{}]}'
    } else { throw "Unexpected or publishing gh command: $args" }
}
try {
    foreach ($scenario in @("success", "wrong-branch", "dirty", "frozen", "unpushed", "existing", "tampered")) {
        $global:idvDraftTestScenario = $scenario
        $global:idvDraftTestCalls = [Collections.Generic.List[string]]::new()
        if ($scenario -eq "tampered") { Write-Fixture $asset "changed bytes" }
        $failure = $null
        try { & (Join-Path $scripts "publish_release.ps1") -NotesFile (Join-Path $fixture "notes.md") | Out-Null } catch { $failure = $_ }
        if ($scenario -eq "success") {
            if ($failure) { throw $failure }
            if (-not ($global:idvDraftTestCalls | Where-Object { $_ -like "release upload*" })) { throw "Missing draft upload" }
            $body = Get-Content -LiteralPath (Join-Path $bundle "release-body.txt") -Raw
            if ($body.IndexOf("## SHA-256") -le $body.IndexOf("Full Changelog")) { throw "Checksum section must follow changelog" }
        } else {
            if (-not $failure) { throw "Expected gate to reject $scenario" }
            if ($global:idvDraftTestCalls | Where-Object { $_ -like "*POST*" -or $_ -like "release upload*" }) { throw "Rejected $scenario mutated a draft" }
        }
        Write-Host "PASS: $scenario"
    }
} finally { Set-Location -LiteralPath $originalLocation.Path }
