set -e
NETWORK=mainnet
SUFFIX=near
OWNER=metapool.sputnik-dao.$SUFFIX
META_POOL_CONTRACT=meta-pool.$SUFFIX
CONTRACT_ACC=lockup-meta-pool.near

bash build.sh

WASM=res/lockup_stake_metapool.wasm
export NEAR_ENV=$NETWORK

set -ex

# DEPLOY with init
# near deploy $CONTRACT_ACC $WASM \
#      new "{\"owner_id\":\"$OWNER\", \"meta_pool_contract_id\":\"$META_POOL_CONTRACT\"}"

# first ping
near call $CONTRACT_ACC ping --accountId $CONTRACT_ACC --gas 50000000000000