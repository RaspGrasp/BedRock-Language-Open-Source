@echo off
setlocal EnableDelayedExpansion

:: Check for Administrator privileges
net session >nul 2>&1
if %errorLevel% neq 0 (
    echo Requesting Administrator privileges...
    powershell -Command "Start-Process '%~f0' -Verb RunAs"
    exit /b
)

echo [BedRock] Uninstallation Script
echo -------------------------------

:: 1. Registry Cleanup
echo Removing File Associations...

:: Delete .br extension key
reg delete "HKCR\.br" /f >nul 2>&1
if %errorLevel% equ 0 (
    echo [OK] Removed .br extension association.
) else (
    echo [INFO] .br extension not found or already removed.
)

:: Delete BedRockSource class key
reg delete "HKCR\BedRockSource" /f >nul 2>&1
if %errorLevel% equ 0 (
    echo [OK] Removed BedRockSource class.
) else (
    echo [INFO] BedRockSource class not found or already removed.
)

:: 2. PATH Cleanup
echo.
echo Checking System PATH...

:: Locate brc3.exe to identify the path to remove
set "TARGET_EXE=brc3.exe"
set "FOUND_PATH="

:: Check possible locations (same logic as setup)
if exist "%~dp0%TARGET_EXE%" set "FOUND_PATH=%~dp0%TARGET_EXE%"
if not defined FOUND_PATH if exist "%~dp0compiler\target\release\%TARGET_EXE%" set "FOUND_PATH=%~dp0compiler\target\release\%TARGET_EXE%"
if not defined FOUND_PATH if exist "%~dp0examples\%TARGET_EXE%" set "FOUND_PATH=%~dp0examples\%TARGET_EXE%"
if not defined FOUND_PATH if exist "%~dp0bin\%TARGET_EXE%" set "FOUND_PATH=%~dp0bin\%TARGET_EXE%"

if defined FOUND_PATH (
    echo Found installed binary at: !FOUND_PATH!
    for %%F in ("!FOUND_PATH!") do set "EXE_DIR=%%~dpF"
    :: Remove trailing backslash
    set "EXE_DIR=!EXE_DIR:~0,-1!"
    
    echo Removing directory from System PATH...
    powershell -Command "$dir = '!EXE_DIR!'; $currentPath = [System.Environment]::GetEnvironmentVariable('Path', 'Machine'); $parts = $currentPath -split ';'; $newParts = $parts | Where-Object { $_ -ne $dir -and $_ -ne '' }; $newPath = $newParts -join ';'; if ($currentPath -ne $newPath) { [System.Environment]::SetEnvironmentVariable('Path', $newPath, 'Machine'); Write-Host '[OK] Path removed.' } else { Write-Host '[INFO] Directory not found in PATH.' }"
) else (
    echo [WARNING] bedrockc.exe not found relative to this script.
    echo Cannot automatically determine which directory to remove from PATH.
    echo Please manually check your Environment Variables if needed.
)

:: 3. Refresh Explorer
echo.
echo Refreshing Windows Icon Cache...
ie4uinit.exe -show
taskkill /f /im explorer.exe >nul 2>&1
start explorer.exe

echo.
echo [BedRock] Uninstallation Complete!
pause
