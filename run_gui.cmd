@echo off
rem starts the browser editor and opens it in the default browser (Windows
rem version of run_gui.sh): double-click it in the release folder.
rem extra arguments go to "mb3d gui", e.g. run_gui.cmd "M3Parameter\file.m3p"
rem the port can be changed with: set PORT=8081 ^& run_gui.cmd
setlocal
cd /d "%~dp0"
if "%PORT%"=="" set PORT=8080
if not "%~1"==":open" (
  curl -s -o nul "http://127.0.0.1:%PORT%/" && (
    echo Another mb3d gui is already running on port %PORT%: close its window first,
    echo or start this one on another port: set PORT=8081 ^& run_gui.cmd
    pause
    exit /b 1
  )
)
if "%~1"==":open" goto open
start "" /b cmd /c ""%~f0" :open"
mb3d.exe gui --port %PORT% %*
exit /b

rem waits until the server answers (at most about 20 s), then opens the page
:open
set /a tries=0
:wait
curl -s -o nul "http://127.0.0.1:%PORT%/" && goto show
set /a tries+=1
if %tries% geq 20 goto show
ping -n 2 127.0.0.1 >nul
goto wait
:show
start "" "http://127.0.0.1:%PORT%/"
