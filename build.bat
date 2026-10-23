@echo off
setlocal EnableExtensions

cd /d "%~dp0"

echo [1/3] Checking current version...
for /f "tokens=2 delims==" %%A in ('findstr /r /c:"^version = " Cargo.toml') do set "VERSION=%%~A"

if "%VERSION%"=="" (
    echo ERROR: Could not read version from Cargo.toml.
    exit /b 1
)

echo Version: %VERSION%

echo [2/3] Building release...
cargo build --release
if errorlevel 1 exit /b 1

echo [3/3] Building installer...
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

echo.
echo Build complete:
echo Version: %VERSION%
echo dist\xemanh2-%VERSION%-setup.exe

exit /b 0
