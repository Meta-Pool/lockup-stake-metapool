use near_sdk::AccountId;

use crate::U256;
pub const TGAS: u64 = 1_000_000_000_000;

/// returns amount * numerator/denominator
pub fn mul_div(amount: u128, numerator: u128, denominator: u128) -> u128 {
    return (U256::from(amount) * U256::from(numerator) / U256::from(denominator)).as_u128();
}

/// verify if it a lockup account
pub fn is_lockup_account(account_id: &str) -> bool{
    account_id.ends_with(".lockup.near") 
    || account_id.ends_with(".lockupy.testnet") 
}

/// assert it is not a lockup account
pub fn assert_is_lockup_account(account_id: &AccountId) {
    assert!(is_lockup_account(&account_id.as_str()),"only .lockup.near account can be used here");
}
