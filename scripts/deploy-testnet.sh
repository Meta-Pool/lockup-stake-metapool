set -e
bash build.sh

NETWORK=testnet
POOL_DOT_NETWORK=pool.$NETWORK
META_POOL_CONTRACT=meta-v2.$POOL_DOT_NETWORK
MASTER_ACC=$META_POOL_CONTRACT
CONTRACT_ACC=lockup.$MASTER_ACC

export NEAR_ENV=$NETWORK
echo $NETWORK, $CONTRACT_ACC

WASM=res/lockup_stake_metapool.wasm
OWNER=test-narwallets.$NETWORK

# FIRST DEPLOY
## delete acc
# echo "Delete $CONTRACT_ACC? are you sure? Ctrl-C to cancel"
# read input
# near delete $CONTRACT_ACC $MASTER_ACC
# near create-account $CONTRACT_ACC --masterAccount $MASTER_ACC
#  near deploy $CONTRACT_ACC $WASM \
#      new "{\"owner_id\":\"$OWNER\", \"meta_pool_contract_id\":\"$META_POOL_CONTRACT\"}" \
#      --accountId $MASTER_ACC
#  exit    
## set params@meta set_params
#meta set_params
## deafult 4 pools
##meta default_pools_testnet

## test
#near call $CONTRACT_ACC set_busy "{\"value\":false}" --accountId $CONTRACT_ACC --depositYocto 1

# set contract busy to make sure we're not upgrading in the middle of a cross-contract call
# set -ex
# near call $CONTRACT_ACC set_busy '{"value":true}' --accountId $OPERATOR_ACC --depositYocto 1
# set -e

# RE-DEPLOY, code only
near deploy $CONTRACT_ACC $WASM  --accountId $MASTER_ACC --networkId $NETWORK

#near call $CONTRACT_ACC set_busy '{"value":false}' --accountId $OPERATOR_ACC --depositYocto 1
