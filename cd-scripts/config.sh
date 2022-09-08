### NOIS SPECIFIC PARAMS #########
GIT_CONTRACTS_URL="https://github.com/noislabs/nois-contracts"
GIT_CONTRACTS_TAG="v0.2.0"

NOIS_NODE_URL=http://6553qqb75pb27eg2ff5lqvrpso.ingress.akash.pro:80
NOIS_BINARY_NAME=wasmd
NOIS_CHAIN_ID=nois-testnet-000
NOIS_DENOM=unois
NOIS_GAS_PRICES=1
NOIS_LOCAL_KEYRING_KEY=seondary
NOIS_FAUCET_URL="http://faucet.noislabs.com/credit"
#comment out NOIS_DRAND_CONTRACT_ADDRESS to deploy a new one
NOIS_DRAND_CONTRACT_ADDRESS=nois1s66zhks8v3fm24974crzxufh7w6ktt69jq8e3zt8q7cyvr52vlqqg5eym6



#### CHAIN SPECIFIC PARAMS #######
##juno
NODE_URL=https://rpc.uni.juno.deuslabs.fi:443
CHAIN_ID=uni-3
BINARY_NAME=junod
DENOM=ujunox
#FEES=1000000#deprecated
GAS_PRICES=0.025
PREFIX=juno
LOCAL_KEYRING_KEY=chain-key
RELAYER_CHAIN_NAME=$CHAIN_ID
#Comment the FAUCET_URL declaration line if you don't want or cannot use a faucet
FAUCET_URL="https://faucet.uni.juno.deuslabs.fi/credit"
RELAYER_IBC_SRC_CONNECTION=connection-268
RELAYER_IBC_DEST_CONNECTION=connection-5
###################################

#### CHAIN SPECIFIC PARAMS #######
###stargaze
#NODE_URL=https://rpc.elgafar-1.stargaze-apis.com:443
#CHAIN_ID=elgafar-1
#BINARY_NAME=starsd
#DENOM=ustars
#GAS_PRICES=0.025
#PREFIX=stars
#LOCAL_KEYRING_KEY=chain-key
#RELAYER_CHAIN_NAME=elgafar-1
#
#RELAYER_IBC_SRC_CONNECTION=connection-0
#RELAYER_IBC_DEST_CONNECTION=connection-11
#Comment the FAUCET_URL declaration line if you don't want or cannot use a faucet
#FAUCET_URL=""
##################################

### IBC SETUP####################
RELAYER_IBC_VERSION=nois-v1
RELAYER_DOCKER_IMAGE=noislabs/nois-relayer:$CHAIN_ID
#RELAYER_IBC_SRC_CONNECTION=connection-268
#RELAYER_IBC_DEST_CONNECTION=connection-5