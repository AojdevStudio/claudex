#!/usr/bin/env bash
set -euo pipefail

if (( $# == 0 )); then
  echo "usage: $0 PACKAGE..." >&2
  exit 2
fi

apt_options=(
  -o Acquire::Retries=3
  -o Acquire::http::Timeout=20
  -o Acquire::https::Timeout=20
  -o DPkg::Lock::Timeout=60
)

install_packages() {
  sudo env DEBIAN_FRONTEND=noninteractive timeout --kill-after=10s 120s \
    apt-get "${apt_options[@]}" install --yes "$@"
}

# GitHub-hosted runners already carry a current package index. Prefer it so a
# transient Ubuntu mirror failure cannot block an otherwise reproducible build.
if install_packages "$@"; then
  exit 0
fi

for attempt in 1 2; do
  if sudo timeout --kill-after=10s 120s apt-get "${apt_options[@]}" update &&
    install_packages "$@"; then
    exit 0
  fi

  if (( attempt < 2 )); then
    echo "APT refresh attempt ${attempt} failed; retrying in 10 seconds" >&2
    sleep 10
  fi
done

echo "APT setup failed after the cached index and 2 bounded refresh attempts" >&2
exit 1
