@echo off
setlocal

rem Use the MSVC compiler from the installed Visual Studio. This avoids
rem inheriting an incompatible global CC value (for example "zig cc").
set "VSWHERE=%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe"
if not exist "%VSWHERE%" (
  echo Visual Studio Build Tools were not found. Install the Desktop development with C++ workload.
  exit /b 1
)

set "VSDIR="
for /f "usebackq tokens=*" %%i in (`"%VSWHERE%" -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath`) do set "VSDIR=%%i"
if not defined VSDIR (
  echo Visual Studio C++ tools were not found. Install the Desktop development with C++ workload.
  exit /b 1
)

call "%VSDIR%\Common7\Tools\VsDevCmd.bat" -arch=x64 -host_arch=x64
if errorlevel 1 exit /b %errorlevel%

set "CC=cl"
set "CXX=cl"

rem Dependencies are installed explicitly with "pnpm install --frozen-lockfile".
rem Do not let pnpm purge and reinstall them implicitly when the dev command starts.
set "npm_config_verify_deps_before_run=false"
set "PNPM_CONFIG_VERIFY_DEPS_BEFORE_RUN=false"

call pnpm tauri dev
