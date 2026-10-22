$ErrorActionPreference = "Stop"

$path = Join-Path $PSScriptRoot "Cargo.toml"
$content = [IO.File]::ReadAllText($path)

$match = [regex]::Match(
    $content,
    '(?m)^version\s*=\s*"(\d+)\.(\d+)\.(\d+)"\s*$'
)

if (-not $match.Success) {
    throw "Cannot find package version in Cargo.toml"
}

$major = [int]$match.Groups[1].Value
$minor = [int]$match.Groups[2].Value
$patch = [int]$match.Groups[3].Value + 1
$version = "$major.$minor.$patch"

$replacement = 'version = "' + $version + '"'
$updated = $content.Substring(0, $match.Index) +
    $replacement +
    $content.Substring($match.Index + $match.Length)

[IO.File]::WriteAllText(
    $path,
    $updated,
    [Text.UTF8Encoding]::new($false)
)

[IO.File]::WriteAllText(
    (Join-Path $PSScriptRoot ".release-version"),
    $version,
    [Text.UTF8Encoding]::new($false)
)

Write-Host "Version updated: $version"
