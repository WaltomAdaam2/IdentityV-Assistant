param(
    [string] $BundleDirectory,
    [string] $Version,
    [string] $NotesFile
)
$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$config = Get-Content -LiteralPath (Join-Path $repoRoot "src-tauri/tauri.conf.json") -Raw -Encoding UTF8 | ConvertFrom-Json
if (-not $Version) { $Version = $config.version }
if ($Version -notmatch '^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$') { throw "Invalid release version." }
if (-not $BundleDirectory) { $BundleDirectory = Join-Path $repoRoot "src-tauri/target/release/bundle/nsis" }
$installer = Join-Path $BundleDirectory ($config.productName + "_${Version}_x64-setup.exe")
$signaturePath = $installer + ".sig"
if (-not (Test-Path -LiteralPath $installer -PathType Leaf) -or -not (Test-Path -LiteralPath $signaturePath -PathType Leaf)) {
    throw "Signed x64 NSIS installer is missing. Build successfully before generating update metadata."
}
$signature = (Get-Content -LiteralPath $signaturePath -Raw).Trim()
if (-not $signature) { throw "Installer signature is empty." }
$signatureText = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($signature))
$trustedComment = ($signatureText -split "`n" | Where-Object { $_.StartsWith('trusted comment: ') }) -join ''
if (($trustedComment.TrimEnd("`r") -split "`t") -notcontains "version:$Version") {
    throw "Installer signature must contain signed version:$Version. Rebuild with Tauri CLI 2.12 or newer."
}
$assetName = [Guid]::NewGuid().ToString("N") + ".exe"
$assetPath = Join-Path $BundleDirectory $assetName
Copy-Item -LiteralPath $installer -Destination $assetPath -Force
Copy-Item -LiteralPath $signaturePath -Destination ($assetPath + ".sig") -Force
$hash = (Get-FileHash -LiteralPath $assetPath -Algorithm SHA256).Hash.ToLowerInvariant()
$notes = if ($NotesFile) { Get-Content -LiteralPath $NotesFile -Raw -Encoding UTF8 } else { "Identity V Assistant v$Version" }
$commit = & git -C $repoRoot rev-parse HEAD
if ($LASTEXITCODE -ne 0) { throw "Could not read build commit." }
$changes = & git -C $repoRoot status --porcelain
if ($LASTEXITCODE -ne 0) { throw "Could not read source status." }
$manifest = [ordered]@{
    version = $Version
    notes = $notes.Trim()
    pub_date = [DateTime]::UtcNow.ToString("yyyy-MM-ddTHH:mm:ssZ")
    commit = $commit.Trim()
    source_clean = -not [bool]$changes
    platforms = @{ "windows-x86_64" = [ordered]@{
        url = "https://github.com/WaltomAdaam2/desktop-releases/releases/download/v$Version/$assetName"
        signature = $signature
        sha256 = $hash
        size = (Get-Item -LiteralPath $assetPath).Length
    } }
}
$utf8 = [Text.UTF8Encoding]::new($false)
[IO.File]::WriteAllText((Join-Path $BundleDirectory "latest.json"), ($manifest | ConvertTo-Json -Depth 5), $utf8)
[IO.File]::WriteAllText((Join-Path $BundleDirectory "SHA256SUMS.txt"), "$hash  $assetName`n", $utf8)
Write-Host "Signed update artifacts: $BundleDirectory"
