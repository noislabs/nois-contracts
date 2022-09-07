### NOIS SPECIFIC PARAMS #########
GIT_CONTRACTS_URL="https://github.com/noislabs/nois-contracts"
GIT_CONTRACTS_TAG="v0.2.0"

NOIS_NODE_URL=http://6553qqb75pb27eg2ff5lqvrpso.ingress.akash.pro:80
NOIS_BINARY_NAME=wasmd
NOIS_CHAIN_ID=nois-testnet-000
NOIS_DENOM=unois
NOIS_LOCAL_KEYRING_KEY=seondary
NOIS_FAUCET_URL="http://mbnquketflch91cp592h6ao3mk.ingress.d3akash.cloud/credit"

### IBC SETUP####################
RELAYER_IBC_VERSION=nois-v1
RELAYER_DOCKER_IMAGE=noislabs/nois-relayer:0.0.1
RELAYER_IBC_SRC_CONNECTION=connection-268
RELAYER_IBC_DEST_CONNECTION=connection-5

#### CHAIN SPECIFIC PARAMS #######
NODE_URL=https://rpc.uni.juno.deuslabs.fi:443
CHAIN_ID=uni-3
BINARY_NAME=junod
DENOM=ujunox
FEES=1000000
PREFIX=juno
LOCAL_KEYRING_KEY=juno-key
RELAYER_CHAIN_NAME=uni
#Comment the FAUCET_URL declaration line if you don't want or cannot use a faucet
FAUCET_URL="https://faucet.uni.juno.deuslabs.fi/credit"
##################################

