set -e
NETWORK=mainnet
SUFFIX=near
OWNER=metapool.sputnik-dao.$SUFFIX
META_POOL_CONTRACT=meta-pool.$SUFFIX
MASTER_ACC=$META_POOL_CONTRACT
CONTRACT_ACC=lockup.$MASTER_ACC

bash build.sh

WASM=res/lockup_stake_metapool.wasm
export NEAR_ENV=$NETWORK

set -ex

# FIRST DEPLOY
## delete acc
# echo "Delete $CONTRACT_ACC? are you sure? Ctrl-C to cancel"
# read input
# near delete $CONTRACT_ACC $MASTER_ACC
near create-account $CONTRACT_ACC --masterAccount $MASTER_ACC --initialBalance 10
near deploy $CONTRACT_ACC $WASM \
     new "{\"owner_id\":\"$OWNER\", \"meta_pool_contract_id\":\"$META_POOL_CONTRACT\"}" \
     --accountId $MASTER_ACC
 exit    

# RE-DEPLOY, code only
# echo $NETWORK, $CONTRACT_ACC
# near deploy $CONTRACT_ACC $WASM  --accountId $MASTER_ACC --networkId $NETWORK

# update price
near call $CONTRACT_ACC ping --accountId $OWNER