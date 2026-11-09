@echo off
setlocal
cd /d "%~dp0"
if not exist "live-code-room-server.exe" (
  echo Не найден live-code-room-server.exe. Распакуйте всю папку программы.
  pause
  exit /b 1
)
live-code-room-server.exe
if errorlevel 1 (
  echo.
  echo Сервер остановился с ошибкой. Проверьте доступ к папке portable-data.
)
pause
