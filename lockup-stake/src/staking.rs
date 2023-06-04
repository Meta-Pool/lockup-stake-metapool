use near_sdk::PromiseResult;
use near_sdk::is_promise_success;
use near_sdk::log;

use crate::ext_contract;
use crate::utils::mul_div;
use crate::utils::TGAS;
use crate::*;

// NOTE:
// NEAR_WALLET, DEFAULT_GAS_ATTACHED on deposit_and_stake: 125*TGAS
/// The foundation's near-core/lockup-contract USES:
///  50TGAS for DEPOSIT
///  75TGAS for DEPOSIT_AND_STAKE
/// Requires 175TGAS for withdraw_all_from_staking_pool - https://github.com/near/core-contracts/blob/dad58eb5f968c25913e746028ad63980506f5890/lockup/src/owner.rs#L256
pub const META_POOL_DEPOSIT_AND_STAKE_GAS: u64 = 30 * TGAS;
pub const AFTER_DEPOSIT_AND_STAKE_GAS: u64 = 5 * TGAS;

pub const META_POOL_WITHDRAW_GAS: u64 = 20 * TGAS;
pub const AFTER_WITHDRAW_GAS: u64 = 5 * TGAS;

pub const META_POOL_UNSTAKE_SHARES_GAS: u64 = 20 * TGAS;
pub const AFTER_UNSTAKE_SHARES_GAS: u64 = 5 * TGAS;

/// Interface for Meta Pool
#[ext_contract(ext_metapool)]
trait mp {
    fn deposit_and_stake(&mut self) -> U128;
    fn withdraw(&mut self, amount: U128) -> Promise;
    fn unstake_shares(&mut self, shares: U128) -> U128;
}
/// Interface for the contract itself.
#[ext_contract(ext_self)]
pub trait SelfContract {
    /// A callback to check the result of the staking action.
    /// In case the stake failed, this callback rollbacks changes
    fn after_deposit_and_stake(
        &mut self,
        account_id: AccountId,
        amount: Balance,
        included_a_deposit: bool,
    );
    fn after_withdraw(&mut self, account_id: AccountId, total_requested: Balance);
    fn after_unstake_shares(&mut self, account_id: AccountId, num_shares: Balance);
}

#[near_bindgen]
impl StakingContract {
    /// Just deposits the attached amount into account.unstaked - does not stake
    #[payable]
    pub fn deposit(&mut self) {
        self.internal_deposit();
    }

    /// Withdraws the entire unstaked balance from the predecessor account.
    /// It's only allowed if the `unstake` action was not performed in the four most recent epochs.
    pub fn withdraw_all(&mut self) -> Promise {
        let account_id = env::predecessor_account_id();
        let account = self.internal_get_account(&account_id);
        self.perform_withdraw(&account_id, account.unstaked)
    }

    /// Withdraws the non staked balance for given account.
    /// It's only allowed if the `unstake` action was not performed in the four most recent epochs.
    pub fn withdraw(&mut self, amount: U128) -> Promise {
        self.perform_withdraw(&env::predecessor_account_id(), amount.into())
    }

    fn perform_withdraw(&mut self, account_id: &AccountId, amount: Balance) -> Promise {
        assert!(amount > 0, "Withdrawal amount should be positive");

        let mut account = self.internal_get_account(&account_id);
        assert!(
            account.unstaked_available_epoch_height <= env::epoch_height(),
            "The unstaked balance is not yet available due to unstaking delay"
        );

        if account.unstaked >= amount {
            // local balance is enough
            account.unstaked -= amount;
            self.internal_save_account(&account_id, &account);

            log!(
                "@{} LOCAL withdrawing {}. New unstaked balance is {}",
                account_id,
                amount,
                account.unstaked
            );

            Promise::new(account_id.clone()).transfer(amount)
        } else if account.unstaked + account.unstaked_in_metapool >= amount {
            // we have to ask first Meta Pool for the withdraw
            ext_metapool::withdraw(
                account.unstaked_in_metapool.into(),
                self.meta_pool_contract_id.clone(),
                0,
                Gas(META_POOL_WITHDRAW_GAS),
            )
            .then(ext_self::after_withdraw(
                account_id.clone(),
                amount,
                env::current_account_id(),
                0,
                Gas(AFTER_WITHDRAW_GAS),
            ))
        } else {
            panic!("Not enough unstaked balance to withdraw");
        }
    }
    #[private]
    // continues after previous fn
    pub fn after_withdraw(&mut self, account_id: AccountId, total_requested: Balance) {
        // WARN: This is a callback after-cross-contract-call method
        // busy locks must be saved false in the state, this method SHOULD NOT PANIC
        self.contract_busy = false;
        let mut account = self.internal_get_account(&account_id);
        if !is_promise_success() {
            // withdraw at meta pool failed, but can not panic
            log!(
                "ERROR! @{} asking for METAPOOL withdraw {} FAILED",
                account_id,
                account.unstaked_in_metapool,
            );
        } else {
            log!(
                "@{} asking for METAPOOL withdraw {} OK!, original user request {}",
                account_id,
                account.unstaked_in_metapool,
                total_requested
            );
            // retrieved from metapool is: account.unstaked_in_metapool
            // total requested is: total_requested
            if total_requested > account.unstaked_in_metapool {
                // unstaked_in_metapool was only partially contributing
                let extra = total_requested - account.unstaked_in_metapool;
                account.unstaked_in_metapool = 0;
                account.unstaked = account.unstaked.saturating_sub(extra)
            } else {
                // what we retrieved covers requested
                account.unstaked_in_metapool =
                    account.unstaked_in_metapool.saturating_sub(total_requested)
            }
            self.internal_save_account(&account_id, &account);

            Promise::new(account_id.clone()).transfer(total_requested);
        }
    }

    /// Deposits the attached amount into the inner account of the predecessor and stakes it.
    /// Note: The foundation-s near-core/lockup-contract USES 50GAS for this call
    #[payable]
    pub fn deposit_and_stake(&mut self) -> Promise {
        self.internal_deposit();
        self.perform_stake(env::predecessor_account_id(), env::attached_deposit(), true)
    }

    /// Stakes all available unstaked balance from the inner account of the predecessor.
    pub fn stake_all(&mut self) -> Promise {
        let account_id = env::predecessor_account_id();
        let account = self.internal_get_account(&account_id);
        self.perform_stake(account_id, account.unstaked, false)
    }

    /// Stakes the given amount from the inner account of the predecessor.
    /// The inner account should have enough unstaked balance.
    pub fn stake(&mut self, amount: U128) -> Promise {
        self.perform_stake(env::predecessor_account_id(), amount.into(), false)
    }
    /// Stakes the given amount from the balance at account.unstaked
    /// The account should have enough unstaked balance.
    /// calls Meta Pool to stake
    fn perform_stake(
        &mut self,
        account_id: AccountId,
        amount: u128,
        included_a_deposit: bool,
    ) -> Promise {
        let amount: Balance = amount.into();
        self.internal_register_staking(amount);
        // call meta pool
        self.assert_not_busy();
        self.contract_busy = true;
        ext_metapool::deposit_and_stake(
            self.meta_pool_contract_id.clone(),
            amount,
            Gas(META_POOL_DEPOSIT_AND_STAKE_GAS),
        )
        .then(ext_self::after_deposit_and_stake(
            account_id,
            amount,
            included_a_deposit,
            env::current_account_id(),
            0,
            Gas(AFTER_DEPOSIT_AND_STAKE_GAS),
        ))
    }
    #[private]
    // continues after previous fn
    pub fn after_deposit_and_stake(
        &mut self,
        account_id: AccountId,
        amount: Balance,
        included_a_deposit: bool,
    ) {
        // WARN: This is a callback after-cross-contract-call method
        // busy locks must be saved false in the state, this method SHOULD NOT PANIC
        // SO DO NOT USE `#[callback]num_shares:U128` arguments, decode the return value manually

        self.contract_busy = false;
        // Check promise result and det the received_nears from the promise result.
        match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),

            PromiseResult::Successful(value) => {
                if let Ok(num_shares) = near_sdk::serde_json::from_slice::<U128>(&value) {
                    // register shares received
                    let mut account = self.internal_get_account(&account_id);
                    account.stake_shares += num_shares.0;
                    self.internal_save_account(&account_id, &account);
                    self.total_stake_shares += num_shares.0;
                } else {
                    // promise ok but no result? -- should not happen
                    log!("UNEXPECTED ERROR: promise ok but no result!",);
                }
            }

            PromiseResult::Failed => {
                // stake at meta pool failed, ROLLBACK
                self.rollback_internal_stake(account_id.clone(), amount);
                if included_a_deposit {
                    // roll back deposit registration
                    self.rollback_internal_deposit(&account_id, amount);
                    // return NEARs to user
                    Promise::new(account_id).transfer(amount);
                }
            }
        };

    }

    /// Unstakes all staked balance from the inner account of the predecessor.
    /// The new total unstaked balance will be available for withdrawal in four epochs.
    pub fn unstake_all(&mut self) -> Promise {
        let account_id = env::predecessor_account_id();
        let account = self.internal_get_account(&account_id);
        self.inner_unstake_shares(&account_id, account.stake_shares)
    }

    /// Unstakes the given amount (in NEARs) from the inner account of the predecessor.
    /// The inner account should have enough staked balance.
    /// The new total unstaked balance will be available for withdrawal in four epochs.
    pub fn unstake(&mut self, amount: U128) -> Promise {
        let amount: Balance = amount.into();
        let shares = mul_div(amount, ONE_NEAR, self.share_near_price);
        self.inner_unstake_shares(&env::predecessor_account_id(), shares)
    }

    fn inner_unstake_shares(&mut self, account_id: &AccountId, num_shares: u128) -> Promise {
        assert!(num_shares > 0, "Unstaking share amount should be positive");

        assert!(
            self.total_staked_balance > 0,
            "The contract doesn't have staked balance"
        );

        let account = self.internal_get_account(&account_id);
        assert!(
            account.stake_shares >= num_shares,
            "Not enough staked balance to unstake"
        );

        log!(
            "@{} unstaking {} staking shares. owned shares {} ",
            account_id,
            num_shares,
            account.stake_shares
        );

        // call meta pool
        self.assert_not_busy();
        self.contract_busy = true;
        ext_metapool::unstake_shares(
            num_shares.into(),
            self.meta_pool_contract_id.clone(),
            0,
            Gas(META_POOL_UNSTAKE_SHARES_GAS),
        )
        .then(ext_self::after_unstake_shares(
            account_id.clone(),
            num_shares,
            env::current_account_id(),
            0,
            Gas(AFTER_UNSTAKE_SHARES_GAS),
        ))
    }
    #[private]
    // continues after previous fn
    pub fn after_unstake_shares(&mut self, account_id: AccountId, num_shares: Balance) {
        // WARN: This is a callback after-cross-contract-call method
        // busy locks must be saved false in the state, this method SHOULD NOT PANIC
        // SO DO NOT USE `#[callback]received_nears:U128` arguments, decode the return value manually
        self.contract_busy = false;

        // Check promise result and det the received_nears from the promise result.
        match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),

            PromiseResult::Successful(value) => {
                if let Ok(received_nears) = near_sdk::serde_json::from_slice::<U128>(&value) {
                    // register the successful unstake share
                    let mut account = self.internal_get_account(&account_id);
                    account.stake_shares -= num_shares;
                    account.unstaked_in_metapool += received_nears.0;
                    account.unstaked_available_epoch_height =
                        env::epoch_height() + NUM_EPOCHS_TO_UNLOCK;
                    self.internal_save_account(&account_id, &account);
                    log!(
                        "unstake shares at meta pool OK! account {}, shares {}, received_near {}",
                        account_id,
                        num_shares,
                        received_nears.0
                    );

                    self.total_staked_balance -= received_nears.0;
                    self.total_stake_shares -= num_shares;

                    log!(
                        "Contract total staked balance is {}. Total number of shares {}",
                        self.total_staked_balance,
                        self.total_stake_shares
                    );
                } else {
                    // promise ok but no result? -- should not happen
                    log!("UNEXPECTED ERROR: promise ok but no result!",);
                }
            }

            PromiseResult::Failed => {
                // unstake shares at meta pool failed!
                log!(
                    "ERR: unstake shares at meta pool failed! account {}, shares {}",
                    account_id,
                    num_shares
                );
            }
        };
    }
}
