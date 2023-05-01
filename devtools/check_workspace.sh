#!/bin/bash
set -o errexit -o nounset -o pipefail
command -v shellcheck >/dev/null && shellcheck "$0"

cargo check
cargo check --features governance_owned

cargo test

cargo clippy --all-targets
cargo clippy --all-targets --features governance_owned
