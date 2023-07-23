use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::json_types::U128;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{
    env, ext_contract, near_bindgen, AccountId, Balance, Gas,
    Promise
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

construct_uint! {
    /// 256-bit unsigned integer.
    #[derive(BorshSerialize, BorshDeserialize)]
    pub struct U256(4);
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct StakingContract {
    pub owner_id: AccountId,
    /// The total amount of shares. It should be equal to the total amount of shares across all
    /// accounts.
    pub total_stake_shares: NumStakeShares,
    /// reserved for future use
    pub reserved: Balance,
    /// Persistent map from an account ID to the corresponding account.
    pub accounts: UnorderedMap<AccountId, Account>,
    /// Whether the staking is paused.
    /// When paused, the account unstakes everything (stakes 0) and doesn't restake.
    /// It doesn't affect the staking shares or reward distribution.
    /// Pausing is useful for node maintenance. Only the owner can pause and resume staking.
    /// The contract is not paused by default.
    pub paused: bool,
    /// Avoid re-entrance risks
    pub contract_busy: bool,

    pub meta_pool_contract_id: AccountId,
    // how many nears a share (stNEAR) is worth (should be get from Meta Pool on ping)
    pub share_near_price: Balance,
    // meta pool fee should be get from Meta Pool on ping
    pub meta_pool_fee_bp: u16,
}

impl Default for StakingContract {
    fn default() -> Self {
        panic!("Staking contract should be initialized before usage")
    }
}

#[near_bindgen]
impl StakingContract {
    /// Initializes the contract with the given owner_id, initial staking public key (with ED25519
    /// curve) and initial reward fee fraction that owner charges for the validation work.
    ///
    /// The entire current balance of this contract will be used to stake. This allows contract to
    /// always maintain staking shares that can't be unstaked or withdrawn.
    /// It prevents inflating the price of the share too much.
    #[init]
    pub fn new(
        owner_id: AccountId,
        meta_pool_contract_id: AccountId,
    ) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        assert!(
            env::is_valid_account_id(owner_id.as_bytes()),
            "The owner account ID is invalid"
        );
        Self {
            owner_id,
            reserved: 0,
            total_stake_shares: 0,
            accounts: UnorderedMap::new(b"a"),
            paused: false,
            contract_busy: false,
            meta_pool_contract_id,
            share_near_price: ONE_NEAR,
            meta_pool_fee_bp: 400,
        }
    }

    pub fn assert_not_busy(&self) {
        assert!(!self.contract_busy, "Contract is busy. Try again later");
    }

    pub fn set_not_busy(&mut self) {
        self.assert_owner();
        self.contract_busy=false;
    }

}
