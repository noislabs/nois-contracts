#!/bin/sh

NODE_URL=https://rpc.uni.juno.deuslabs.fi:443
BINARY_NAME=junod
DENOM=ujunox
FEES=1000000

echo "Store proxy"
NOIS_PROXY_CODE_ID=$($BINARY_NAME tx wasm store artifacts/nois_proxy.wasm --from juno-key --chain-id uni-3   --gas=auto --gas-adjustment 1.4  --fees=$FEES$DENOM --broadcast-mode=block --node=$NODE_URL -y |yq -r ".logs[0].events[1].attributes[0].value")

echo "Instantiate proxy"
NOIS_PROXY_CONTRACT_ADDRESS=$($BINARY_NAME tx wasm instantiate $NOIS_PROXY_CODE_ID '{}'  --label=nois-proxy --no-admin --from juno-key --chain-id uni-3   --gas=auto --gas-adjustment 1.4  --fees=$FEES$DENOM --broadcast-mode=block --node=$NODE_URL  -y |yq -r '.logs[0].events[0].attributes[0].value' )
echo "NOIS_PROXY_CONTRACT_ADDRESS: $NOIS_PROXY_CONTRACT_ADDRESS"
 
echo "Store demo"
NOIS_DEMO_CODE_ID=$($BINARY_NAME tx wasm store artifacts/nois_demo.wasm --from juno-key --chain-id uni-3   --gas=auto --gas-adjustment 1.4  --fees=$FEES$DENOM --broadcast-mode=block --node=$NODE_URL -y |yq -r ".logs[0].events[1].attributes[0].value")
 
echo "Instantiate demo"
NOIS_DEMO_CONTRACT_ADDRESS=$($BINARY_NAME tx wasm instantiate $NOIS_DEMO_CODE_ID '{"nois_proxy": "'"$NOIS_PROXY_CONTRACT_ADDRESS"'"}'  --label=nois-demo --no-admin --from juno-key --chain-id uni-3   --gas=auto --gas-adjustment 1.4  --fees=$FEES$DENOM --broadcast-mode=block --node=$NODE_URL -y |yq -r '.logs[0].events[0].attributes[0].value' )
echo "NOIS_DEMO_CONTRACT_ADDRESS: $NOIS_DEMO_CONTRACT_ADDRESS"
