. scripts/.vars.sh
#/// Requires 125 TGas (5 * BASE_GAS) https://github.com/near/core-contracts/blob/3f3170fce91ff4d8c6ee9d15683f2d4dfe1275cf/lockup/src/owner.rs#L213C5-L213C41
near call $LOCKUP_ACC withdraw_from_staking_pool {\"amount\":\"2$NEARS\"} --accountId $OWNER --gas 125$TGAS
#/// Requires 175 TGas (7 * BASE_GAS) https://github.com/near/core-contracts/blob/3f3170fce91ff4d8c6ee9d15683f2d4dfe1275cf/lockup/src/owner.rs#L259
near call $LOCKUP_ACC withdraw_all_from_staking_pool --accountId $OWNER --gas 175$TGAS
