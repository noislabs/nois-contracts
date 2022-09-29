#!/bin/bash
set -x
#PREREQS
# 0 You have already run the deploy-contracts script

SCRIPT_DIR="cd-scripts"
KEYRING_KEY_NAME="deployment-key"

cd $SCRIPT_DIR

chain=elgafar-1
contract=nois-demo

NOIS_DEMO_CONTRACT_ADDRESS=$(cat config.yaml|yq -r '.chains[]| select(.name=="'"$chain"'").wasm.contracts[]| select(.name=="'"$contract"'").address' )
BINARY_NAME=($(yq -r '.chains[]| select(.name=="'"$chain"'").binary_name' config.yaml))
NODE_URL=($(yq -r '.chains[]| select(.name=="'"$chain"'").rpc[0]' config.yaml))
CHAIN_ID=($(yq -r '.chains[]| select(.name=="'"$chain"'").chain_id' config.yaml))
FAUCET_URL=($(yq -r '.chains[]| select(.name=="'"$chain"'").faucet' config.yaml))
DENOM=($(yq -r '.chains[]| select(.name=="'"$chain"'").denom' config.yaml))
GAS_PRICES=($(yq -r '.chains[]| select(.name=="'"$chain"'").gas_price' config.yaml))

if [ "$FAUCET_URL" == "~" ] ;
        then echo "$chain : Info: Faucet is not relevant here";
        else echo "$chain : Trying to add credit for chain '$CHAIN_ID' with faucet '$FAUCET_URL'";
          BECH_ADDR=$($BINARY_NAME keys show $KEYRING_KEY_NAME -a ) 
          curl -XPOST -H 'Content-type: application/json' -d "{\"address\":\"$BECH_ADDR\",\"denom\":\"$DENOM\"}" $FAUCET_URL
          echo "$chain - $contract : querying new balance ..."
          $BINARY_NAME query bank balances $BECH_ADDR --node=$NODE_URL | yq -r '.balances' 
    fi

#$BINARY_NAME tx wasm execute artifacts/nois_demo.wasm --from $KEYRING_KEY_NAME --chain-id $CHAIN_ID   --gas=auto --gas-adjustment 1.4  --fees=$FEES$DENOM --broadcast-mode=block --node=$NODE_URL -y
declare -i TTL
TTL='200'

while true
do
   timestamp=$(date +%s)
   result="null"
   echo "$timestamp"
   $BINARY_NAME tx wasm execute $NOIS_DEMO_CONTRACT_ADDRESS  '{"estimate_pi": {"job_id": "'"$timestamp"'"}}'  --from $KEYRING_KEY_NAME --chain-id $CHAIN_ID   --gas=auto --gas-adjustment 1.4  --gas-prices=$GAS_PRICES$DENOM --broadcast-mode=block --node=$NODE_URL -y >/dev/null
   SECONDS=0
   i=0
   while [ "$result" == "null" ] && [ "$i" -lt "$TTL" ] 
   do
     result=$($BINARY_NAME query wasm  contract-state  smart $NOIS_DEMO_CONTRACT_ADDRESS  '{"result": {"job_id":"'"$timestamp"'"}}'  --node=$NODE_URL |yq -r '.data')
     #echo "attempt: $i"
     sleep 1
     let i++
   done
   if [ "$i" -eq "$TTL" ] ;
     then
        echo "randomness took longer than TTL";
   fi
   if [ "$result" != "null" ];
     then
        echo "randomness took $SECONDS seconds";
   fi
   
done
