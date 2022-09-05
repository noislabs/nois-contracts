#!/bin/bash

set -ex
#PREREQS
# 1 You need to install the specific binary of the chain you want to deploy to.
# 2 You need to create a key in your local key ring and add some tokens to it (faucet if testnet)


NODE_URL=https://rpc.uni.juno.deuslabs.fi:443
CHAIN_ID=uni-3
BINARY_NAME=junod
DENOM=ujunox
FEES=1000000
LOCAL_KEYRING_KEY=juno-key

#Comment the FAUCET_URL declaration line if you don't want or cannot use a faucet
FAUCET_URL="https://faucet.uni.juno.deuslabs.fi/credit"


if [ -z ${FAUCET_URL+x} ]; then echo "Info: Faucet is not relevant here";
else echo "Trying to add credit with faucet '$FAUCET_URL'";
  BECH_ADDR=$($BINARY_NAME keys show $LOCAL_KEYRING_KEY -a ) 
  curl -XPOST -H 'Content-type: application/json' -d "{\"address\":\"$BECH_ADDR\",\"denom\":\"$DENOM\"}" $FAUCET_URL
  echo "querying new balance ..."
  $BINARY_NAME query bank balances $BECH_ADDR --node=$NODE_URL | yq -r '.balances' 

fi

echo "Store proxy"
NOIS_PROXY_CODE_ID=$($BINARY_NAME tx wasm store artifacts/nois_proxy.wasm --from $LOCAL_KEYRING_KEY --chain-id $CHAIN_ID   --gas=auto --gas-adjustment 1.4  --fees=$FEES$DENOM --broadcast-mode=block --node=$NODE_URL -y |yq -r ".logs[0].events[1].attributes[0].value")

echo "Instantiate proxy"
NOIS_PROXY_CONTRACT_ADDRESS=$($BINARY_NAME tx wasm instantiate $NOIS_PROXY_CODE_ID '{}'  --label=nois-proxy --no-admin --from $LOCAL_KEYRING_KEY --chain-id $CHAIN_ID   --gas=auto --gas-adjustment 1.4  --fees=$FEES$DENOM --broadcast-mode=block --node=$NODE_URL  -y |yq -r '.logs[0].events[0].attributes[0].value' )
echo "NOIS_PROXY_CONTRACT_ADDRESS: $NOIS_PROXY_CONTRACT_ADDRESS"
 
echo "Store demo"
NOIS_DEMO_CODE_ID=$($BINARY_NAME tx wasm store artifacts/nois_demo.wasm --from $LOCAL_KEYRING_KEY --chain-id $CHAIN_ID   --gas=auto --gas-adjustment 1.4  --fees=$FEES$DENOM --broadcast-mode=block --node=$NODE_URL -y |yq -r ".logs[0].events[1].attributes[0].value")
 
echo "Instantiate demo"
NOIS_DEMO_CONTRACT_ADDRESS=$($BINARY_NAME tx wasm instantiate $NOIS_DEMO_CODE_ID '{"nois_proxy": "'"$NOIS_PROXY_CONTRACT_ADDRESS"'"}'  --label=nois-demo --no-admin --from $LOCAL_KEYRING_KEY --chain-id $CHAIN_ID   --gas=auto --gas-adjustment 1.4  --fees=$FEES$DENOM --broadcast-mode=block --node=$NODE_URL -y |yq -r '.logs[0].events[0].attributes[0].value' )
echo "NOIS_DEMO_CONTRACT_ADDRESS: $NOIS_DEMO_CONTRACT_ADDRESS"
