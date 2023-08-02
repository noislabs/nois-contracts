#!/bin/bash
set -o errexit -o nounset -o pipefail
command -v shellcheck >/dev/null && shellcheck "$0"

# This file is used for local testing. Once you update Rust files, run this to prepare them for the
# integration tests. Then you can run the integration tests.

# go to root dir regardless of where it was run
SCRIPT_DIR="$(realpath "$(dirname "$0")")"
cd "${SCRIPT_DIR}/.."

# compile all contracts
for C in ./contracts/*/
do
  echo "Compiling $(basename "$C")..."
  (cd "$C" && RUSTFLAGS='-C link-arg=-s' cargo build --release --target wasm32-unknown-unknown --locked)
done

# move them to the internal dir inside tests
mkdir -p ./tests/internal

for SRC in ./target/wasm32-unknown-unknown/release/*.wasm; do
  FILENAME=$(basename "$SRC")
  if command -v wasm-opt >/dev/null ; then
    # We use --signext-lowering to avoid sign extension problems with CosmWasm < 1.3.
    wasm-opt -Os --signext-lowering "$SRC" -o "./tests/internal/$FILENAME"
    chmod -x "./tests/internal/$FILENAME"
  else
    cp "$SRC" "./tests/internal/$FILENAME"
  fi
done

ls -l ./tests/internal
