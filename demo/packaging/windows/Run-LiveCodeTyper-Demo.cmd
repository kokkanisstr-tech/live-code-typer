@echo off
setlocal
cd /d "%~dp0"
if not exist "live-code-typer-demo.exe" (
  echo Не найден live-code-typer-demo.exe. Распакуйте всю папку программы.
  pause
  exit /b 1
)
live-code-typer-demo.exe
