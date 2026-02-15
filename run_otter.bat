@echo off
REM Otter - Easy Launcher for Windows
REM This script makes it simple to run Otter without manual setup

echo ================================================
echo      Otter - Decentralized Private Chat
echo ================================================
echo.

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
