#!/bin/bash


#PREREQS
# 0 You need Install yq and fetch
# 1 For fetch to work, Get a github token and run export GITHUB_OAUTH_TOKEN=
# 2 You need to install the specific binary of the chain you want to deploy to.
# 3 Edit the CHAIN SPECIFIC PARAMS
# 4 If the chain to deploy to is mainnet or no faucet can be provisioned in the params then you need to fill your key with some tokens
SCRIPT_DIR="cd-scripts"
cd $SCRIPT_DIR
. config.sh

if [ -f "env_secrets.sh" ]; then
    source env_secrets.sh;
else
  echo "some secrets are missing. create env_secrets.sh file"
fi
echo $GITHUB_OAUTH_TOKEN

echo "downloading contracts from $GIT_CONTRACTS_URL from release $GIT_CONTRACTS_TAG"
fetch --repo="$GIT_CONTRACTS_URL" --tag="$GIT_CONTRACTS_TAG" --release-asset="nois_demo.wasm" ../artifacts
fetch --repo="$GIT_CONTRACTS_URL" --tag="$GIT_CONTRACTS_TAG" --release-asset="nois_proxy.wasm" ../artifacts
fetch --repo="$GIT_CONTRACTS_URL" --tag="$GIT_CONTRACTS_TAG" --release-asset="nois_terrand.wasm" ../artifacts


echo "add chain_key if it does not exist"
$BINARY_NAME keys show $LOCAL_KEYRING_KEY >/dev/null || echo $CHAIN_MNEMONIC | $BINARY_NAME keys  add  $LOCAL_KEYRING_KEY --recover 


if [ -z ${FAUCET_URL+x} ]; then echo "Info: Faucet is not relevant here";
else echo "Trying to add credit for chain '$CHAIN_ID' with faucet '$FAUCET_URL'";
  BECH_ADDR=$($BINARY_NAME keys show $LOCAL_KEYRING_KEY -a ) 
  curl -XPOST -H 'Content-type: application/json' -d "{\"address\":\"$BECH_ADDR\",\"denom\":\"$DENOM\"}" $FAUCET_URL
  echo "querying new balance ..."
  $BINARY_NAME query bank balances $BECH_ADDR --node=$NODE_URL | yq -r '.balances' 
fi

if [ -z ${NOIS_FAUCET_URL+x} ]; then echo "Info: Faucet is not relevant here";
else echo "Trying to add credit for chain '$NOIS_CHAIN_ID' with faucet '$NOIS_FAUCET_URL'";
  BECH_ADDR=$($NOIS_BINARY_NAME keys show $NOIS_LOCAL_KEYRING_KEY -a ) 
  curl -XPOST -H 'Content-type: application/json' -d "{\"address\":\"$BECH_ADDR\",\"denom\":\"$NOIS_DENOM\"}" $NOIS_FAUCET_URL
  echo "querying new balance ..."
  $NOIS_BINARY_NAME query bank balances $BECH_ADDR --node=$NOIS_NODE_URL | yq -r '.balances' 
fi

if [ -z ${NOIS_DRAND_CONTRACT_ADDRESS+x} ]; 
then 
  echo "storing and instantiating contracts to $NOIS_CHAIN_ID"

  echo "Store nois-drand"
  NOIS_DRAND_CODE_ID=$($NOIS_BINARY_NAME tx wasm store ../artifacts/nois_terrand.wasm --from $NOIS_LOCAL_KEYRING_KEY --chain-id $NOIS_CHAIN_ID   --gas=auto --gas-adjustment 1.9  --gas-prices=$NOIS_GAS_PRICES$NOIS_DENOM --broadcast-mode=block --node=$NOIS_NODE_URL -y |yq -r ".logs[0].events[1].attributes[0].value")
  
  echo "Instantiate nois-drand"
  NOIS_DRAND_CONTRACT_ADDRESS=$($NOIS_BINARY_NAME tx wasm instantiate $NOIS_DRAND_CODE_ID '{"test_mode":false}'   --label=nois-drand --no-admin --from $NOIS_LOCAL_KEYRING_KEY --chain-id $NOIS_CHAIN_ID   --gas=auto --gas-adjustment 1.9  --gas-prices=$NOIS_GAS_PRICES$NOIS_DENOM --broadcast-mode=block --node=$NOIS_NODE_URL  -y |yq -r '.logs[0].events[0].attributes[0].value' )
  echo "NOIS_DRAND_CONTRACT_ADDRESS: $NOIS_DRAND_CONTRACT_ADDRESS"
else
  echo "Skipping deployment to nois because NOIS_DRAND_CONTRACT_ADDRESS is set";
fi



echo "storing and instantiating contracts to $CHAIN_ID"

echo "Store proxy"
NOIS_PROXY_CODE_ID=$($BINARY_NAME tx wasm store ../artifacts/nois_proxy.wasm --from $LOCAL_KEYRING_KEY --chain-id $CHAIN_ID   --gas=auto --gas-adjustment 1.9  --gas-prices=$GAS_PRICES$DENOM --broadcast-mode=block --node=$NODE_URL -y |yq -r ".logs[0].events[1].attributes[0].value")

echo "Instantiate proxy"
NOIS_PROXY_CONTRACT_ADDRESS=$($BINARY_NAME tx wasm instantiate $NOIS_PROXY_CODE_ID '{}'  --label=nois-proxy --no-admin --from $LOCAL_KEYRING_KEY --chain-id $CHAIN_ID   --gas=auto --gas-adjustment 1.9  --gas-prices=$GAS_PRICES$DENOM --broadcast-mode=block --node=$NODE_URL  -y |yq -r '.logs[0].events[0].attributes[0].value' )
echo "NOIS_PROXY_CONTRACT_ADDRESS: $NOIS_PROXY_CONTRACT_ADDRESS"
 
echo "Store demo"
NOIS_DEMO_CODE_ID=$($BINARY_NAME tx wasm store ../artifacts/nois_demo.wasm --from $LOCAL_KEYRING_KEY --chain-id $CHAIN_ID   --gas=auto --gas-adjustment 1.9  --gas-prices=$GAS_PRICES$DENOM  --broadcast-mode=block --node=$NODE_URL -y |yq -r ".logs[0].events[1].attributes[0].value")
 
echo "Instantiate demo"
NOIS_DEMO_CONTRACT_ADDRESS=$($BINARY_NAME tx wasm instantiate $NOIS_DEMO_CODE_ID '{"nois_proxy": "'"$NOIS_PROXY_CONTRACT_ADDRESS"'"}'  --label=nois-demo --no-admin --from $LOCAL_KEYRING_KEY --chain-id $CHAIN_ID   --gas=auto --gas-adjustment 1.9  --gas-prices=$GAS_PRICES$DENOM  --broadcast-mode=block --node=$NODE_URL -y |yq -r '.logs[0].events[0].attributes[0].value' )
echo "NOIS_DEMO_CONTRACT_ADDRESS: $NOIS_DEMO_CONTRACT_ADDRESS"

echo "generating relayer config"
cp relayer/nois-relayer-config-template.yaml relayer/nois-relayer-config.yaml
sed -i '' "s#TEMPLATE_RELAYER_CHAIN_NAME#${RELAYER_CHAIN_NAME}#" relayer/nois-relayer-config.yaml
sed -i '' "s#TEMPLATE_CHAIN_ID#${CHAIN_ID}#" relayer/nois-relayer-config.yaml
sed -i '' "s#TEMPLATE_CHAIN_PREFIX#${PREFIX}#" relayer/nois-relayer-config.yaml
sed -i '' "s#TEMPLATE_CHAIN_DENOM#${DENOM}#" relayer/nois-relayer-config.yaml
sed -i '' "s#TEMPLATE_NOIS_PROXY_CONTRACT_ADDRESS#${NOIS_PROXY_CONTRACT_ADDRESS}#" relayer/nois-relayer-config.yaml
sed -i '' "s#TEMPLATE_CHAIN_FAUCET_URL#${FAUCET_URL}#" relayer/nois-relayer-config.yaml
sed -i '' "s#TEMPLATE_CHAIN_NODE_URL#${NODE_URL}#" relayer/nois-relayer-config.yaml
sed -i '' "s#TEMPLATE_NOIS_DRAND_CONTRACT_ADDRESS#${NOIS_DRAND_CONTRACT_ADDRESS}#" relayer/nois-relayer-config.yaml


echo "building relayer docker"
cd relayer
docker build --build-arg CHAIN_NAME=$CHAIN_ID -t $RELAYER_DOCKER_IMAGE .

if [ -z ${RELAYER_IBC_SRC_CONNECTION+x} ] || [ -z ${RELAYER_IBC_DEST_CONNECTION+x} ]  ; 
then echo "WARN: RELAYER_IBC_SRC_CONNECTION or RELAYER_IBC_DEST_CONNECTION are not defined ";
     echo "Creating a connection... please note the src and connection ids and define those variables accordingly"
     docker run  -e RELAYER_MNEMONIC="$RELAYER_MNEMONIC" $RELAYER_DOCKER_IMAGE ibc-setup connect
else echo "Info: RELAYER_IBC_SRC_CONNECTION and RELAYER_IBC_DEST_CONNECTION are set, skipping connection creation"; 
fi

echo "creating IBC channel"
docker run  -e RELAYER_MNEMONIC="$RELAYER_MNEMONIC" $RELAYER_DOCKER_IMAGE ibc-setup channel --src-connection=$RELAYER_IBC_SRC_CONNECTION --dest-connection=$RELAYER_IBC_DEST_CONNECTION --src-port=wasm.$NOIS_PROXY_CONTRACT_ADDRESS --dest-port=wasm.$NOIS_DRAND_CONTRACT_ADDRESS --version=$RELAYER_IBC_VERSION

echo "pushing relayer docker so it is ready to be deployed"
docker push $RELAYER_DOCKER_IMAGE



if [ -z ${DISCORD_WEBHOOK+x} ] ; 
then echo "WARN: Skipping notification because DISCORD_WEBHOOK is not set ";
else echo "notify on discord"

generate_post_data()
{
  cat <<EOF

  {
    "$NOIS_CHAIN_ID": [
      {
        "name": "drand-nois",
        "address": "$NOIS_DRAND_CONTRACT_ADDRESS",
        "code_version": "$GIT_CONTRACTS_TAG",
        "code_id": "$NOIS_DRAND_CODE_ID"
      }
    ],
    "ibc": {
      "relayer_docker_image": "$RELAYER_DOCKER_IMAGE",
      "ibc_src_connection": "$RELAYER_IBC_SRC_CONNECTION",
      "ibc_dest_connection": "$RELAYER_IBC_DEST_CONNECTION",
      "ibc_src_port": "wasm.$NOIS_PROXY_CONTRACT_ADDRESS",
      "ibc_dest_port": "wasm.$NOIS_DRAND_CONTRACT_ADDRESS",
      "ibc_version": "$RELAYER_IBC_VERSION"
    },
    "$CHAIN_ID": [
      {
        "contract_name": "nois-proxy",
        "address": "$NOIS_PROXY_CONTRACT_ADDRESS",
        "code_version": "$GIT_CONTRACTS_TAG",
        "code_id": "$NOIS_PROXY_CODE_ID"
      },
      {
        "contract_name": "nois-demo",
        "address": "$NOIS_DEMO_CONTRACT_ADDRESS",
        "code_version": "$GIT_CONTRACTS_TAG",
        "code_id": "$NOIS_DEMO_CODE_ID"
      }
      
    ]
  }
EOF
}
generated_data=$(generate_post_data)
echo $generated_data | jq -r . >  ../generated_$CHAIN_ID.json
message=$(echo $generated_data | jq -R .)

curl  -H "Content-Type: application/json" \
-H "Content-Type:application/json" \
-XPOST -d "{\"content\":$message}" \
$DISCORD_WEBHOOK; 
fi

