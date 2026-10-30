param([Parameter(Mandatory = $true)][string] $NotesFile)
$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "file_hash.ps1")
$NotesFile = (Resolve-Path -LiteralPath $NotesFile).Path
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$notes = Get-Content -LiteralPath $NotesFile -Raw -Encoding UTF8
if (-not $notes.Trim()) { throw "Release notes must not be empty." }
Set-Location -LiteralPath $repoRoot
if (-not (Get-Command gh -ErrorAction SilentlyContinue)) { throw "Install GitHub CLI and run gh auth login first." }
$branch = & git branch --show-current
if ($LASTEXITCODE -ne 0 -or $branch -ne "main") { throw "Publish from main only." }
$changes = & git status --porcelain
if ($LASTEXITCODE -ne 0 -or $changes) { throw "Commit all source changes before building and publishing." }
$commit = & git rev-parse HEAD
if ($LASTEXITCODE -ne 0) { throw "Cannot read HEAD." }
$remoteCommit = & git ls-remote origin refs/heads/main
if ($LASTEXITCODE -ne 0 -or -not $remoteCommit -or ($remoteCommit -split '\s+')[0] -ne $commit) {
    throw "Push main before publishing; remote main must match the built commit."
}
$bundle = Join-Path $repoRoot "src-tauri/target/release/bundle/nsis"
$manifestPath = Join-Path $bundle "latest.json"
$manifest = Get-Content -LiteralPath $manifestPath -Raw -Encoding UTF8 | ConvertFrom-Json
$version = (Get-Content -LiteralPath "src-tauri/tauri.conf.json" -Raw -Encoding UTF8 | ConvertFrom-Json).version
if ($manifest.version -ne $version -or $manifest.commit -ne $commit -or -not $manifest.source_clean) {
    throw "Build artifacts are stale or were built with uncommitted changes. Run the build again from clean main."
}
$tag = "v$version"
$repo = "WaltomAdaam2/IdentityV-Assistant"
# Tags in the source repository identify the actual verified build commit.
$releaseCommit = $commit
# Enumerate through a successful API request: a network failure must not look like an absent release.
$releases = & gh api --paginate "repos/$repo/releases?per_page=100" --jq '.[].tag_name'
if ($LASTEXITCODE -ne 0) { throw "Could not inspect existing releases." }
if ($releases -contains $tag) { throw "$tag already exists (possibly a draft from a previous attempt). Inspect it manually; this script never overwrites releases." }
$tags = & git ls-remote "https://github.com/$repo.git" "refs/tags/$tag"
if ($LASTEXITCODE -ne 0 -or $tags) { throw "Cannot publish: tag exists or remote inspection failed." }
$platform = $manifest.platforms.'windows-x86_64'
$assetName = ($platform.url -split '/')[-1]
if ($assetName -cne "IdentityV-Assistant-${version}-Windows-x64-Setup.exe") { throw "Installer filename must match the product, version and platform." }
$asset = Join-Path $bundle $assetName
if ($platform.url -ne "https://github.com/$repo/releases/download/$tag/$assetName" -or
    (Get-Content -LiteralPath (Join-Path $bundle "SHA256SUMS.txt") -Raw).Trim() -ne ($platform.sha256 + "  " + $assetName) -or
    (Get-ReleaseFileHash $asset) -ne $platform.sha256 -or
    (Get-Item -LiteralPath $asset).Length -ne $platform.size -or
    (Get-Content -LiteralPath ($asset + '.sig') -Raw).Trim() -ne $platform.signature) {
    throw "Release artifacts no longer match the signed build metadata. Rebuild before publishing."
}
$manifest.notes = $notes.Trim()
$manifest.pub_date = [DateTime]::UtcNow.ToString("yyyy-MM-ddTHH:mm:ssZ")
[IO.File]::WriteAllText($manifestPath, ($manifest | ConvertTo-Json -Depth 5), [Text.UTF8Encoding]::new($false))
$releaseBody = Join-Path $bundle "release-body.txt"
$checksums = @($asset, ($asset + ".sig"), $manifestPath, (Join-Path $bundle "SHA256SUMS.txt")) | ForEach-Object {
    (Get-ReleaseFileHash $_) + "  " + [IO.Path]::GetFileName($_)
}
$body = $notes.Trim() + "`n`n## SHA-256`n`n" + '```text' + "`n" + ($checksums -join "`n") + "`n" + '```' + "`n"
[IO.File]::WriteAllText($releaseBody, $body, [Text.UTF8Encoding]::new($false))
$isPrerelease = $version.Contains('-')
$channelArguments = if ($isPrerelease) { @('--prerelease') } else { @() }
& gh release create $tag $asset ($asset + ".sig") $manifestPath (Join-Path $bundle "SHA256SUMS.txt") --repo $repo --target $releaseCommit --draft --title "Identity V Assistant v$version" --notes-file $releaseBody @channelArguments
if ($LASTEXITCODE -ne 0) { throw "Draft creation/upload failed or outcome is unknown. Inspect GitHub Releases before retrying. No existing release will be overwritten." }
$latestArgument = if ($isPrerelease) { '--latest=false' } else { '--latest=true' }
& gh release edit $tag --repo $repo --draft=false $latestArgument
if ($LASTEXITCODE -ne 0) { throw "Publishing did not confirm success. Inspect the existing draft/release on GitHub; do not create another one." }
& gh release view $tag --repo $repo --json url,assets
if ($LASTEXITCODE -ne 0) { throw "Release was published but final verification failed. Inspect it on GitHub." }
