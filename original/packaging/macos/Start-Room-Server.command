#!/bin/sh
set -eu
HERE="$(cd "$(dirname "$0")" && pwd)"
if "$HERE/live-code-room-server"; then
  exit 0
else
  status=$?
  echo "Сервер остановился с ошибкой. Проверьте доступ к папке portable-data."
  read -r -p "Нажмите Enter, чтобы закрыть это окно..." _
  exit "$status"
fi
