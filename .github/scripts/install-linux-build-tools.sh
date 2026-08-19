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

for attempt in 1 2 3; do
  if sudo timeout --kill-after=10s 120s apt-get "${apt_options[@]}" update &&
    sudo env DEBIAN_FRONTEND=noninteractive timeout --kill-after=10s 120s \
      apt-get "${apt_options[@]}" install --yes "$@"; then
    exit 0
  fi

  if (( attempt < 3 )); then
    echo "APT setup attempt ${attempt} failed; retrying in 10 seconds" >&2
    sleep 10
  fi
done

echo "APT setup failed after 3 bounded attempts" >&2
exit 1
