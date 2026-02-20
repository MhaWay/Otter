@echo off
REM Otter - Easy Launcher for Windows
REM This script makes it simple to run Otter without manual setup

echo ================================================
echo      Otter - Decentralized Private Chat
echo ================================================
echo.

REM Load environment variables from .env file if it exists
if exist .env (
    echo Loading environment variables from .env...
    for /f "delims== tokens=1,2" %%A in ('.env') do (
        if not "%%A"==" " if not "%%A:~0,1%% "=="#" set "%%A=%%B"
    )
) else (
    echo Note: .env file not found. If you need to use Google OAuth, please:
    echo   1. Copy .env.example to .env
    echo   2. Add your Google OAuth credentials
    echo   See SETUP_OAUTH.md for instructions.
    echo.
)

REM Check if otter.exe exists
if exist otter.exe (
    echo Found otter.exe
    echo.
    echo Starting Otter...
    echo.
    otter.exe
) else (
    echo ERROR: otter.exe not found in current directory
    echo.
    echo Please ensure you have:
    echo 1. Downloaded the complete Otter release package
    echo 2. Extracted all files to the same directory
    echo 3. You are running this script from that directory
    echo.
    echo If you need to build from source:
    echo   cargo build --release -p otter-cli
    echo   copy target\release\otter.exe .
    echo.
    pause
    exit /b 1
)

REM If we get here, otter exited
echo.
echo Otter has exited.
pause
