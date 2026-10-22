@echo off
setlocal EnableExtensions

cd /d "%~dp0"

echo [1/4] Checking working tree...
for /f "delims=" %%A in ('git status --porcelain -uall') do (
    echo ERROR: Working tree is not clean.
    echo %%A
    exit /b 1
)

echo [2/4] Incrementing patch version...
powershell -NoProfile -ExecutionPolicy Bypass -Command ^
  '$p = "Cargo.toml"; $s = [IO.File]::ReadAllText($p); $m = [regex]::Match($s, "(?m)^version\s*=\s*""(\d+)\.(\d+)\.(\d+)""\s*$"); if (!$m.Success) { throw "Cannot find package version in Cargo.toml" }; $v = [int]$m.Groups[3].Value + 1; $new = "$($m.Groups[1].Value).$($m.Groups[2].Value).$v"; $s = $s.Substring(0,$m.Index) + ("version = ""$new""") + $s.Substring($m.Index + $m.Length); [IO.File]::WriteAllText($p, $s, [Text.UTF8Encoding]::new($false)); Set-Content -Path ".release-version" -Value $new -NoNewline'

if errorlevel 1 exit /b 1

set /p VERSION=<.release-version
del .release-version

echo Version: %VERSION%

echo [3/4] Building release...
cargo build --release
if errorlevel 1 exit /b 1

echo [4/4] Building installer...
if not exist dist mkdir dist

set "ISCC=%LocalAppData%\Programs\Inno Setup 6\ISCC.exe"
if not exist "%ISCC%" (
    echo ERROR: Inno Setup compiler not found:
    echo %ISCC%
    exit /b 1
)

set "XEMANH2_VERSION=%VERSION%"
"%ISCC%" /DAppVersion=%VERSION% installer.iss
if errorlevel 1 exit /b 1

echo.
echo Release complete:
echo dist\xemanh2-%VERSION%-setup.exe
exit /b 0
