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

set -ex
#near call $CONTRACT_ACC set_not_busy --accountId $OWNER
near view $CONTRACT_ACC get_total_staked_balance 
near view $CONTRACT_ACC get_total_stake_shares
near view $CONTRACT_ACC get_account "{\"account_id\":\"$OPERATOR_ACC\"}"
near view $META_POOL_CONTRACT get_account "{\"account_id\":\"$CONTRACT_ACC\"}"

# near call $META_POOL_CONTRACT test_can_withdraw --accountId $OPERATOR_ACC
# near call $META_POOL_CONTRACT withdraw_all --accountId $CONTRACT_ACC


#near call $CONTRACT_ACC deposit  --accountId $OPERATOR_ACC --deposit 2
#near call $CONTRACT_ACC deposit_and_stake  --accountId $OPERATOR_ACC --deposit 2 --gas $NEAR_WALLET_DEFAULT_STAKE_TGAS

#near call $CONTRACT_ACC ping --accountId $OPERATOR_ACC
# near view $CONTRACT_ACC get_account "{\"account_id\":\"$OPERATOR_ACC\"}"

# near call $CONTRACT_ACC withdraw "{\"amount\":\"2$NEARS\"}" --accountId $OPERATOR_ACC
# near call $CONTRACT_ACC withdraw "{\"amount\":\"1$NEARS\"}" --accountId $OPERATOR_ACC

#near call $CONTRACT_ACC stake_all  --accountId $OPERATOR_ACC --gas $NEAR_WALLET_DEFAULT_STAKE_TGAS
#near call $CONTRACT_ACC stake_all "{\"amount\":\"1$NEARS\"}" --accountId $OPERATOR_ACC

near call $CONTRACT_ACC unstake_all --accountId $OPERATOR_ACC --gas $NEAR_WALLET_DEFAULT_STAKE_TGAS
#near call $CONTRACT_ACC withdraw_all --accountId $OPERATOR_ACC --gas $NEAR_WALLET_DEFAULT_STAKE_TGAS

#near state $CONTRACT_ACC
near view $CONTRACT_ACC get_total_staked_balance 
near view $CONTRACT_ACC get_total_stake_shares
near view $CONTRACT_ACC get_account "{\"account_id\":\"$OPERATOR_ACC\"}"
near view $META_POOL_CONTRACT get_account "{\"account_id\":\"$CONTRACT_ACC\"}"
