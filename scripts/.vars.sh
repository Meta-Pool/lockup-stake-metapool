set -e
NETWORK=testnet
OWNER=lucio.$NETWORK
OPERATOR_ACC=lucio.$NETWORK
MASTER_ACC=pool.$NETWORK
META_POOL_CONTRACT=meta-v2.$MASTER_ACC
CONTRACT_ACC=lockup.$META_POOL_CONTRACT

SIXZEROS=000000
TGAS=$SIXZEROS$SIXZEROS
NEAR_WALLET_DEFAULT_STAKE_TGAS=125$TGAS;
NEARS=$TGAS$TGAS

export NEAR_ENV=$NETWORK
echo $NEAR_ENV

LOCKUP_ACC="274e981786efcabbe87794f20348c1b2af6e7963.lockupy.testnet"
set -ex
