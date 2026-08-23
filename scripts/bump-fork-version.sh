#!/usr/bin/env bash
# Bump the fork's `-orgoj.N` suffix in Cargo.toml and keep Cargo.lock in sync.
#
# Every change in this fork ships a new orgoj.N, and doing that by hand across
# two files is how the lock file drifts from the manifest.
#
# Usage: scripts/bump-fork-version.sh [new-version]
#   without an argument, N is incremented by one.
set -euo pipefail

cd "$(dirname "$0")/.."

current=$(awk -F'"' '/^version = "/ { print $2; exit }' Cargo.toml)
if [[ -z ${current} ]]; then
    echo "cannot read version from Cargo.toml" >&2
    exit 1
fi

if [[ $# -gt 0 ]]; then
    next=$1
else
    if [[ ! ${current} =~ ^(.*-orgoj\.)([0-9]+)$ ]]; then
        echo "version '${current}' has no -orgoj.N suffix; pass the new version explicitly" >&2
        exit 1
    fi
    next="${BASH_REMATCH[1]}$((BASH_REMATCH[2] + 1))"
fi

if [[ ${next} == "${current}" ]]; then
    echo "version is already ${current}" >&2
    exit 1
fi

python3 - "${current}" "${next}" <<'PY'
import re
import sys

current, next_version = sys.argv[1], sys.argv[2]

with open("Cargo.toml", encoding="utf-8") as fh:
    manifest = fh.read()
updated, count = re.subn(
    rf'^version = "{re.escape(current)}"$',
    f'version = "{next_version}"',
    manifest,
    count=1,
    flags=re.M,
)
if count != 1:
    sys.exit(f"no package version line for {current} in Cargo.toml")
with open("Cargo.toml", "w", encoding="utf-8") as fh:
    fh.write(updated)

# Only the hcom package entry in the lock file, never a dependency that happens
# to share the version string.
with open("Cargo.lock", encoding="utf-8") as fh:
    lock = fh.read()
updated, count = re.subn(
    rf'(\[\[package\]\]\nname = "hcom"\nversion = )"{re.escape(current)}"',
    rf'\1"{next_version}"',
    lock,
    count=1,
)
if count != 1:
    sys.exit(f"no hcom lock entry for {current} in Cargo.lock")
with open("Cargo.lock", "w", encoding="utf-8") as fh:
    fh.write(updated)
PY

echo "${current} -> ${next}"
