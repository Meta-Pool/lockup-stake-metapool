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
# # delete acc
#  echo "Delete $CONTRACT_ACC? are you sure? Ctrl-C to cancel"
#  read input
#  near delete $CONTRACT_ACC $MASTER_ACC --beneficiaryId $MASTER_ACC
#  near create-account $CONTRACT_ACC --masterAccount $MASTER_ACC --initialBalance 10
#  near deploy $CONTRACT_ACC $WASM \
#       new "{\"owner_id\":\"$OWNER\", \"meta_pool_contract_id\":\"$META_POOL_CONTRACT\"}" \
#       --accountId $MASTER_ACC
# exit    


# RE-DEPLOY, code only
near deploy $CONTRACT_ACC $WASM  --accountId $MASTER_ACC --networkId $NETWORK

# update price
near call $CONTRACT_ACC ping --accountId $OWNER
