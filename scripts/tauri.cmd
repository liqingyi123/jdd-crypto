@echo off
setlocal
call "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat" >nul 2>&1
if errorlevel 1 (
  echo [error] Failed to load Visual Studio vcvars64.bat
  echo Install VS 2022 with "Desktop development with C++", then retry.
  exit /b 1
)
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
"%~dp0..\node_modules\.bin\tauri.cmd" %*
exit /b %ERRORLEVEL%
