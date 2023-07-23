. scripts/.vars.sh

near call $LOCKUP_ACC unstake_all --accountId $OWNER --gas 125$TGAS

near call $META_POOL_CONTRACT accelerate_unstake '{}' --accountId $OPERATOR_ACC 
near call $CONTRACT_ACC accelerate_unstake '{}' --accountId $OPERATOR_ACC 
