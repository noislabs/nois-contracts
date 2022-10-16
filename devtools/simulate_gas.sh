#!/bin/bash
set -o errexit -o nounset -o pipefail
command -v shellcheck >/dev/null && shellcheck "$0"

for WASM in ./artifacts/*.wasm; do
  echo "Simulating gas usage for $WASM"
  KEYNAME=$(noisd keys list | head -n 1 | cut -d " " -f3)
  echo "" | noisd tx \
    wasm store "$WASM" \
    --chain-id nois-testnet-003 \
    --node "https://nois.rpc.bccnodes.com:443" \
    --gas auto \
    --gas-adjustment 1.0 \
    --gas-prices 0.05unois \
    --from "$KEYNAME" \
    --broadcast-mode=block 2>&1 | grep "gas estimate"
done
