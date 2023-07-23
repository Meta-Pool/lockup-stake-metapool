use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::json_types::U128;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{
    env, ext_contract, near_bindgen, AccountId, Balance, Gas,
    Promise, assert_one_yocto
};
use uint::construct_uint;

use crate::account::{Account, NumStakeShares};
pub use crate::views::HumanReadableAccount;

mod account;
mod internal;
mod owner;
mod staking;
mod ping;
mod utils;

mod views;

pub const ONE_E24: u128 = 1_000_000_000_000_000_000_000_000;
pub const NEAR: u128 = ONE_E24;
pub const ONE_NEAR: u128 = NEAR;

pub mod test_only;

construct_uint! {
    /// 256-bit unsigned integer.
    #[derive(BorshSerialize, BorshDeserialize)]
    pub struct U256(4);
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct StakingContract {
    pub owner_id: AccountId,
    /// The total amount of shares, should be equal to sum(accounts.shares).
    pub total_stake_shares: NumStakeShares,
    /// Persistent map from an account ID to the corresponding account.
    pub accounts: UnorderedMap<AccountId, Account>,

    // distributed, decentralized staking contract
    pub meta_pool_contract_id: AccountId,
    // how many nears a share (stNEAR) is worth (get from Meta Pool on ping)
    pub share_near_price: Balance,
    // meta pool fee (get from Meta Pool on ping)
    pub meta_pool_fee_bp: u16,
}

impl Default for StakingContract {
    fn default() -> Self {
        panic!("Staking contract should be initialized before usage")
    }
}

#[near_bindgen]
impl StakingContract {
    /// Initializes the contract 
    #[init]
    pub fn new(
        owner_id: AccountId,
        meta_pool_contract_id: AccountId,
    ) -> Self {
        assert!(
            env::is_valid_account_id(owner_id.as_bytes()),
            "The owner account ID is invalid"
        );
        Self {
            owner_id,
            total_stake_shares: 0,
            accounts: UnorderedMap::new(b"a"),
            meta_pool_contract_id,
            share_near_price: ONE_NEAR,
            meta_pool_fee_bp: 400,
        }
    }

    #[payable]
    pub fn set_not_busy(&mut self, account_id:AccountId) {
        self.assert_owner();
        assert_one_yocto();
        let mut acc = self.accounts.get(&account_id).unwrap();
        acc.busy=false;
        self.internal_save_account(&account_id, &acc);

    }

}
