#!/usr/bin/env bash

# context: cargo-release pre-release-hook
# see https://github.com/crate-ci/cargo-release/blob/master/docs/reference.md
# particularly the environment variables NEW_VERSION, DRY_RUN

set -euo pipefail

OUTPUT=(-p CHANGELOG.md)

if $DRY_RUN; then
  OUTPUT=()
fi

set -x

git cliff -v -u -t "$NEW_VERSION" "${OUTPUT[@]}"

