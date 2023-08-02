#!/bin/bash
set -o errexit -o nounset -o pipefail
command -v shellcheck >/dev/null && shellcheck "$0"

cargo check

for contract in nois-drand nois-payment nois-proxy nois-proxy-governance-owned; do
  (
    cd "./contracts/$contract"
    cargo check --features library
  )
done

cargo test

cargo clippy --all-targets
cargo clippy --all-targets --features governance_owned
