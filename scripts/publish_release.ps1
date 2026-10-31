param(
    [Parameter(Mandatory = $true)][string] $NotesFile,
    [ValidateRange(0, [int]::MaxValue)][int] $MergedPullRequest = 0
)
$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "file_hash.ps1")
$NotesFile = (Resolve-Path -LiteralPath $NotesFile).Path
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$notes = Get-Content -LiteralPath $NotesFile -Raw -Encoding UTF8
if (-not $notes.Trim()) { throw "Release notes must not be empty." }
Set-Location -LiteralPath $repoRoot
if (-not (Get-Command gh -ErrorAction SilentlyContinue)) { throw "Install GitHub CLI and run gh auth login first." }
$repo = "WaltomAdaam2/IdentityV-Assistant"
$version = (Get-Content -LiteralPath "src-tauri/tauri.conf.json" -Raw -Encoding UTF8 | ConvertFrom-Json).version
$branch = & git branch --show-current
$snapshotBranch = "SNAPSHOT-v$version"
if ($LASTEXITCODE -ne 0 -or ($branch -cne $snapshotBranch -and -not ($MergedPullRequest -and -not $branch))) {
    throw "Prepare drafts from $snapshotBranch only (a detached verified build is allowed with -MergedPullRequest)."
}
$changes = & git status --porcelain
if ($LASTEXITCODE -ne 0 -or $changes) { throw "Commit all source changes before building and publishing." }
$commit = & git rev-parse HEAD
if ($LASTEXITCODE -ne 0) { throw "Cannot read HEAD." }
if ($MergedPullRequest) {
    $prJson = & gh api "repos/$repo/pulls/$MergedPullRequest"
    if ($LASTEXITCODE -ne 0) { throw "Could not inspect the merged PR." }
    $pr = $prJson | ConvertFrom-Json
    if (-not $pr.merged_at -or $pr.base.ref -cne "main" -or $pr.head.ref -cne $snapshotBranch -or
        $pr.head.sha -ne $commit -or $pr.head.repo.full_name -ne $repo) {
        throw "The merged PR must identify this exact SNAPSHOT build commit and repository."
    }
    $ancestry = & gh api "repos/$repo/compare/${commit}...main" --jq '.status'
    if ($LASTEXITCODE -ne 0 -or $ancestry -notin @("ahead", "identical")) {
        throw "The verified build commit must remain in main's history."
    }
} else {
    $merged = & gh api --paginate "repos/$repo/pulls?state=closed&head=WaltomAdaam2:$snapshotBranch&per_page=100" --jq '.[] | select(.merged_at != null) | .number'
    if ($LASTEXITCODE -ne 0) { throw "Could not inspect the SNAPSHOT branch's PR state." }
    if ($merged) { throw "This SNAPSHOT is frozen. Use -MergedPullRequest only to upload its original verified build after explicit authorization." }
}
$remoteCommit = & gh api "repos/$repo/git/ref/heads/$snapshotBranch" --jq '.object.sha'
if ($LASTEXITCODE -ne 0 -or $remoteCommit -ne $commit) {
    throw "Push the SNAPSHOT branch first; its remote head must match the built commit."
}
$bundle = Join-Path $repoRoot "src-tauri/target/release/bundle/nsis"
$manifestPath = Join-Path $bundle "latest.json"
$manifest = Get-Content -LiteralPath $manifestPath -Raw -Encoding UTF8 | ConvertFrom-Json
if ($manifest.version -ne $version -or $manifest.commit -ne $commit -or -not $manifest.source_clean) {
    throw "Build artifacts are stale or were built with uncommitted changes. Run the build again from the clean SNAPSHOT branch."
}
$tag = "v$version"
# Tags in the source repository identify the actual verified build commit.
$releaseCommit = $commit
# Enumerate through a successful API request: a network failure must not look like an absent release.
$releases = & gh api --paginate "repos/$repo/releases?per_page=100" --jq '.[].tag_name'
if ($LASTEXITCODE -ne 0) { throw "Could not inspect existing releases." }
if ($releases -contains $tag) { throw "$tag already exists (possibly a draft from a previous attempt). Inspect it manually; this script never overwrites releases." }
$tags = & gh api --paginate "repos/$repo/git/matching-refs/tags/$tag" --jq '.[].ref'
if ($LASTEXITCODE -ne 0 -or $tags -contains "refs/tags/$tag") { throw "Cannot create draft: tag exists or remote inspection failed." }
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
# Creating a draft is the final automation step. Only the user may publish it.
$payloadPath = Join-Path $bundle "draft-release.json"
$payload = @{
    tag_name = $tag; target_commitish = $releaseCommit; name = "Identity V Assistant v$version"
    body = $body; draft = $true; prerelease = $isPrerelease; make_latest = "false"
}
[IO.File]::WriteAllText($payloadPath, ($payload | ConvertTo-Json -Depth 5), [Text.UTF8Encoding]::new($false))
& gh api --method POST "repos/$repo/releases" --input $payloadPath --jq '.html_url'
if ($LASTEXITCODE -ne 0) { throw "Draft creation failed or outcome is unknown. Inspect GitHub before retrying; never overwrite an existing release." }
& gh release upload $tag $asset ($asset + ".sig") $manifestPath (Join-Path $bundle "SHA256SUMS.txt") --repo $repo
if ($LASTEXITCODE -ne 0) { throw "Draft asset upload did not confirm success. Inspect the existing draft before resuming." }
$result = & gh release view $tag --repo $repo --json url,isDraft,targetCommitish,assets
if ($LASTEXITCODE -ne 0) { throw "Draft created, but final inspection failed. Check GitHub before retrying." }
$draft = $result | ConvertFrom-Json
if (-not $draft.isDraft -or $draft.targetCommitish -ne $releaseCommit -or $draft.assets.Count -ne 4) {
    throw "Draft verification failed. Do not publish; inspect its state, target commit and assets."
}
Write-Output $result
Write-Host "Draft ready for the user's review and manual publication. No release was published."
