#!/bin/bash

#PREREQS
# 0 You have already run the deploy-contracts script

SCRIPT_DIR="cd-scripts"
cd $SCRIPT_DIR
. config.sh


NOIS_DEMO_CONTRACT_ADDRESS=$(cat generated_data.json |jq -r '."'"$CHAIN_ID"'"[]| select(.contract_name=="nois-demo").address ' )

#$BINARY_NAME tx wasm execute artifacts/nois_demo.wasm --from $LOCAL_KEYRING_KEY --chain-id $CHAIN_ID   --gas=auto --gas-adjustment 1.4  --fees=$FEES$DENOM --broadcast-mode=block --node=$NODE_URL -y
declare -i TTL
TTL='200'

while true
do
   timestamp=$(date +%s)
   result="null"
   echo "$timestamp"
   $BINARY_NAME tx wasm execute $NOIS_DEMO_CONTRACT_ADDRESS  '{"estimate_pi": {"job_id": "'"$timestamp"'"}}'  --from $LOCAL_KEYRING_KEY --chain-id $CHAIN_ID   --gas=auto --gas-adjustment 1.4  --fees=$FEES$DENOM --broadcast-mode=block --node=$NODE_URL -y >/dev/null
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
