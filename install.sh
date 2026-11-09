#!/bin/sh
set -eu

variant=original
if [ "$#" -ge 1 ]; then variant=$1; fi
launch_app=yes
if [ "$#" -ge 2 ]; then launch_app=$2; fi

repository='kokkanisstr-tech/live-code-typer'
case "$variant" in
  original)
    asset_prefix='live-code-typer'
    app_name='LiveCode Typer.app'
    ;;
  demo)
    asset_prefix='live-code-typer-demo'
    app_name='LiveCode Typer Demo.app'
    ;;
  *)
    printf 'Usage: install.sh [original|demo] [--no-open]\n' >&2
    exit 2
    ;;
esac

case "$(uname -m)" in
  arm64|aarch64) platform='macos-apple-silicon' ;;
  x86_64) platform='macos-intel' ;;
  *)
    printf 'This installer supports Apple Silicon and Intel Macs.\n' >&2
    exit 2
    ;;
esac

case "$launch_app" in
  yes|--open) launch_app=yes ;;
  --no-open) launch_app=no ;;
  *)
    printf 'Usage: install.sh [original|demo] [--no-open]\n' >&2
    exit 2
    ;;
esac

release_json=$(curl --fail --silent --show-error --location \
  "https://api.github.com/repos/$repository/releases?per_page=100")
tag=$(printf '%s\n' "$release_json" |
  grep -Eo '"tag_name"[[:space:]]*:[[:space:]]*"[^"]+"' |
  sed -E 's/.*"([^"]+)"$/\1/' |
  grep -E "^$variant-v" |
  head -n 1 || true)

if [ -z "$tag" ]; then
  printf 'No published %s release is available yet. Check https://github.com/%s/releases.\n' "$variant" "$repository" >&2
  exit 1
fi

asset_name="$asset_prefix-$platform.tar.gz"
temporary_directory=$(mktemp -d)
trap 'rm -rf "$temporary_directory"' EXIT HUP INT TERM

curl --fail --silent --show-error --location --retry 2 \
  "https://github.com/$repository/releases/download/$tag/$asset_name" \
  --output "$temporary_directory/$asset_name"
tar -xzf "$temporary_directory/$asset_name" -C "$temporary_directory"

package_directory="$temporary_directory/$asset_prefix-$platform"
if [ ! -d "$package_directory" ]; then
  printf 'The downloaded archive is missing its application folder.\n' >&2
  exit 1
fi

install_directory="$HOME/Applications/LiveCodeTyper/$variant"
mkdir -p "$install_directory"
cp -R "$package_directory/." "$install_directory/"

app_path="$install_directory/$app_name"
if [ ! -d "$app_path" ]; then
  printf 'The downloaded archive is missing %s.\n' "$app_name" >&2
  exit 1
fi

printf 'Installed LiveCode Typer (%s) %s to: %s\n' "$variant" "$tag" "$install_directory"
if [ "$launch_app" = 'yes' ]; then
  open "$app_path"
fi
