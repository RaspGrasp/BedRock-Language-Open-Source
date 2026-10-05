@echo off
setlocal EnableDelayedExpansion

:: Check for Administrator privileges
net session >nul 2>&1
if %errorLevel% neq 0 (
    echo Requesting Administrator privileges...
    powershell -Command "Start-Process '%~f0' -Verb RunAs"
    exit /b
)

echo [BedRock] Configuration Script
echo ------------------------------

:: Locate bedrockc.exe
set "TARGET_EXE=brc3.exe"
set "FOUND_PATH="

:: Check possible locations
:: 1. Current directory
if exist "%~dp0%TARGET_EXE%" set "FOUND_PATH=%~dp0%TARGET_EXE%"
:: 2. Compiler target release (standard build)
if not defined FOUND_PATH if exist "%~dp0compiler\target\release\%TARGET_EXE%" set "FOUND_PATH=%~dp0compiler\target\release\%TARGET_EXE%"
:: 3. Examples directory (found in user env)
if not defined FOUND_PATH if exist "%~dp0examples\%TARGET_EXE%" set "FOUND_PATH=%~dp0examples\%TARGET_EXE%"
if not defined FOUND_PATH if exist "%~dp0bin\%TARGET_EXE%" set "FOUND_PATH=%~dp0bin\%TARGET_EXE%"

if not defined FOUND_PATH (
    echo [ERROR] bedrockc.exe not found!
    echo Please ensure the compiler is built or placed in a known directory.
    pause
    exit /b
)

echo Found bedrockc.exe at: !FOUND_PATH!
for %%F in ("!FOUND_PATH!") do set "EXE_DIR=%%~dpF"
:: Remove trailing backslash
set "EXE_DIR=!EXE_DIR:~0,-1!"

:: 1. Add to System PATH (using PowerShell for safety)
echo Configuring System PATH...
powershell -Command "$dir = '%EXE_DIR%'; $p = [System.Environment]::GetEnvironmentVariable('Path', 'Machine'); if ($p -notlike ('*' + $dir + '*')) { [System.Environment]::SetEnvironmentVariable('Path', $p + ';' + $dir, 'Machine'); Write-Host 'Path updated.' } else { Write-Host 'Path already present.' }"

:: 2. File Association
echo Configuring File Associations...

:: Create or update .br extension
reg add "HKCR\.br" /ve /t REG_SZ /d "BedRockSource" /f >nul

:: Create BedRockSource class
reg add "HKCR\BedRockSource" /ve /t REG_SZ /d "BedRock Source File" /f >nul

:: Icon Integration
:: Use full absolute path to ensure icons appear everywhere (Desktop, Explorer, etc.)
reg add "HKCR\BedRockSource\DefaultIcon" /ve /t REG_SZ /d "\"!FOUND_PATH!\",0" /f >nul

:: Shell Command
reg add "HKCR\BedRockSource\shell\open\command" /ve /t REG_SZ /d "\"!FOUND_PATH!\" \"%%1\"" /f >nul

:: Refresh Icon Cache and Explorer
echo Refreshing Windows Icon Cache...
ie4uinit.exe -show
taskkill /f /im explorer.exe >nul 2>&1
start explorer.exe

echo.
echo BedRock Environment has been successfully configured!
pause
