@echo off
setlocal EnableExtensions

cd /d "%~dp0"

echo [1/6] Checking working tree...
for /f "delims=" %%A in ('git status --porcelain -uall') do (
    echo ERROR: Working tree is not clean.
    echo %%A
    exit /b 1
)

echo [2/6] Incrementing patch version...
powershell -NoProfile -ExecutionPolicy Bypass -Command ^
  '$p = "Cargo.toml"; $s = [IO.File]::ReadAllText($p); $m = [regex]::Match($s, "(?m)^version\s*=\s*""(\d+)\.(\d+)\.(\d+)""\s*$"); if (!$m.Success) { throw "Cannot find package version in Cargo.toml" }; $v = [int]$m.Groups[3].Value + 1; $new = "$($m.Groups[1].Value).$($m.Groups[2].Value).$v"; $s = $s.Substring(0,$m.Index) + ("version = ""$new""") + $s.Substring($m.Index + $m.Length); [IO.File]::WriteAllText($p, $s, [Text.UTF8Encoding]::new($false)); Set-Content -Path ".release-version" -Value $new -NoNewline'

if errorlevel 1 exit /b 1

set /p VERSION=<.release-version
del .release-version

echo Version: %VERSION%

echo [3/6] Building release...
cargo build --release
if errorlevel 1 exit /b 1

echo [4/6] Building installer...
if not exist dist mkdir dist

set "ISCC=%LocalAppData%\Programs\Inno Setup 6\ISCC.exe"
if not exist "%ISCC%" (
    echo ERROR: Inno Setup compiler not found:
    echo %ISCC%
    exit /b 1
)

set "XEMANH2_VERSION=%VERSION%"
"%ISCC%" installer.iss
if errorlevel 1 exit /b 1

echo [5/6] Committing release version...
git add Cargo.toml
git commit -m "chore: release v%VERSION%"
if errorlevel 1 (
    echo ERROR: Failed to commit release version.
    exit /b 1
)

echo [6/6] Uploading GitHub release...
where gh >nul 2>&1
if errorlevel 1 (
    echo ERROR: GitHub CLI ^(gh^) not found.
    exit /b 1
)

gh auth status >nul 2>&1
if errorlevel 1 (
    echo ERROR: GitHub CLI is not authenticated.
    echo Run: gh auth login
    exit /b 1
)

set "TAG=v%VERSION%"
git rev-parse "%TAG%" >nul 2>&1
if errorlevel 1 (
    git tag "%TAG%"
    if errorlevel 1 (
        echo ERROR: Failed to create git tag %TAG%.
        exit /b 1
    )

    git push origin "%TAG%"
    if errorlevel 1 (
        echo ERROR: Failed to push git tag %TAG%.
        exit /b 1
    )
) else (
    echo Git tag %TAG% already exists. Reusing it.
    for /f "delims=" %%A in ('git rev-list -n 1 "%TAG%"') do set "TAG_COMMIT=%%A"
    for /f "delims=" %%A in ('git rev-parse HEAD') do set "HEAD_COMMIT=%%A"
    if not "%TAG_COMMIT%"=="%HEAD_COMMIT%" (
        echo ERROR: Git tag %TAG% points to a different commit.
        echo Tag:  %TAG_COMMIT%
        echo HEAD: %HEAD_COMMIT%
        exit /b 1
    )
)

gh release view "%TAG%" >nul 2>&1
if errorlevel 1 (
    echo Creating GitHub release %TAG%...
    gh release create "%TAG%" "dist\xemanh2-%VERSION%-setup.exe" --title "XemAnh2 %VERSION%" --generate-notes
    if errorlevel 1 (
        echo ERROR: Failed to create GitHub release.
        exit /b 1
    )
) else (
    echo GitHub release %TAG% already exists. Replacing installer...
    gh release upload "%TAG%" "dist\xemanh2-%VERSION%-setup.exe" --clobber
    if errorlevel 1 (
        echo ERROR: Failed to upload installer to GitHub release.
        exit /b 1
    )
)

echo.
echo Release complete:
echo GitHub: %TAG%
echo dist\xemanh2-%VERSION%-setup.exe
exit /b 0
