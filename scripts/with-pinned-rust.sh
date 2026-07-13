#!/bin/sh
set -eu

REPOSITORY_ROOT=$(CDPATH= cd "$(dirname "$0")/.." && pwd)
TOOLCHAIN=$(sed -n 's/^channel = "\([^"]*\)"$/\1/p' "$REPOSITORY_ROOT/rust-toolchain.toml")

if [ -z "$TOOLCHAIN" ] || [ "$(printf '%s\n' "$TOOLCHAIN" | wc -l | tr -d ' ')" -ne 1 ]; then
  echo "unable to read exactly one pinned Rust channel" >&2
  exit 1
fi

RUSTUP_BIN_DIR=${CARGO_HOME:-"$HOME/.cargo"}/bin
PATH="$RUSTUP_BIN_DIR:$PATH"
export PATH

TOOLCHAIN_CARGO=$(rustup which --toolchain "$TOOLCHAIN" cargo)
TOOLCHAIN_BIN=$(dirname "$TOOLCHAIN_CARGO")

PATH="$TOOLCHAIN_BIN:$PATH"
RUSTUP_TOOLCHAIN="$TOOLCHAIN"
export PATH RUSTUP_TOOLCHAIN

exec "$@"
