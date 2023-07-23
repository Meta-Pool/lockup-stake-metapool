use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{Balance, EpochHeight};

/// A type to distinguish between a balance and "stake" shares for better readability.
pub type NumStakeShares = Balance;

/// Inner account data of a delegate.
#[derive(BorshDeserialize, BorshSerialize, Debug, PartialEq, Default)]
pub struct Account {
    /// The unstaked balance. It represents the amount the account has on this contract that
    /// can either be re-staked or withdrawn.
    pub unstaked: Balance,
    /// The unstaked balance in Meta Pool corresponding to this account.
    /// when a delay unstake is initiated, the same order is sent to Meta Pool,
    /// and we register here the amount it should be available there after 4 epochs
    pub unstaked_in_metapool: Balance,
    /// The amount of "stake" shares. Every stake share corresponds to the amount of staked balance.
    /// NOTE: The number of shares should always be less or equal than the amount of staked balance.
    /// This means the price of stake share should always be at least `1`.
    /// The price of stake share is computed in meta pool as `total_staked_balance` / `total_stake_shares`.
    pub stake_shares: NumStakeShares,
    /// The minimum epoch height when the withdrawn is allowed.
    /// This changes after unstaking action, because the amount is still locked for 3 epochs.
    pub unstaked_available_epoch_height: EpochHeight,
}

impl Account {
    pub fn is_empty(&self) -> bool {
        self.unstaked == 0 && self.unstaked_in_metapool == 0 && self.stake_shares == 0
    }
}
