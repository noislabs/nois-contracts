#!/bin/sh

export NODE_URL=https://rpc.uni.juno.deuslabs.fi:443

echo "Store proxy"
export NOIS_PROXY_CODE_ID=$(junod tx wasm store artifacts/nois_proxy.wasm --from juno-key --chain-id uni-3   --gas=auto --gas-adjustment 1.4  --fees=1000000ujunox --broadcast-mode=block --node=$NODE_URL -y |yq -r ".logs[0].events[1].attributes[0].value")

echo "Instantiate proxy"
export NOIS_PROXY_CONTRACT_ADDRESS=$(junod tx wasm instantiate $NOIS_PROXY_CODE_ID '{}'  --label=nois-proxy --no-admin --from juno-key --chain-id uni-3   --gas=auto --gas-adjustment 1.4  --fees=1000000ujunox --broadcast-mode=block --node=$NODE_URL  -y |yq -r '.logs[0].events[0].attributes[0].value' )
echo "NOIS_PROXY_CONTRACT_ADDRESS: $NOIS_PROXY_CONTRACT_ADDRESS"
 

echo "Store demo"
export NOIS_DEMO_CODE_ID=$(junod tx wasm store artifacts/nois_demo.wasm --from juno-key --chain-id uni-3   --gas=auto --gas-adjustment 1.4  --fees=1000000ujunox --broadcast-mode=block --node=$NODE_URL -y |yq -r ".logs[0].events[1].attributes[0].value")
 


echo "Instantiate demo"
export NOIS_DEMO_CONTRACT_ADDRESS=$(junod tx wasm instantiate $NOIS_DEMO_CODE_ID '{"nois_proxy": "'"$NOIS_PROXY_CONTRACT_ADDRESS"'"}'  --label=nois-demo --no-admin --from juno-key --chain-id uni-3   --gas=auto --gas-adjustment 1.4  --fees=1000000ujunox --broadcast-mode=block --node=$NODE_URL -y |yq -r '.logs[0].events[0].attributes[0].value' )
echo "NOIS_DEMO_CONTRACT_ADDRESS: $NOIS_DEMO_CONTRACT_ADDRESS"
