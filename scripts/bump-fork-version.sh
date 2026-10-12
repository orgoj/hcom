#!/usr/bin/env bash
# Bump a fork version suffix (X.Y.Z-<fork>.N) in Cargo.toml and keep Cargo.lock
# in sync.
#
# A fork ships a new N per change, and doing that by hand across two files is
# how the lock file drifts from the manifest. The fork label is read from the
# current version, so nothing here is tied to one fork.
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
    if [[ ! ${current} =~ ^(.*-[A-Za-z0-9_]+\.)([0-9]+)$ ]]; then
        echo "version '${current}' has no fork suffix to increment; pass the new version explicitly" >&2
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
