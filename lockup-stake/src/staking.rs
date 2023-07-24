use near_sdk::is_promise_success;
use near_sdk::json_types::U64;
use near_sdk::log;
use near_sdk::PromiseResult;

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
pub const AFTER_STAKE_FOR_LOCKUP_GAS: u64 = 5 * TGAS;

pub const META_POOL_WITHDRAW_GAS: u64 = 10 * TGAS;
pub const AFTER_WITHDRAW_GAS: u64 = 5 * TGAS;

pub const META_POOL_UNSTAKE_SHARES_GAS: u64 = 20 * TGAS;
pub const AFTER_UNSTAKE_SHARES_GAS: u64 = 5 * TGAS;

/// Interface for Meta Pool
#[ext_contract(ext_metapool)]
trait mp {
    fn stake_for_lockup(&mut self, lockup_account_id: String) -> U128;
    fn unstake_from_lockup_shares(&mut self, lockup_account_id: String, shares: U128) -> U64;
    fn withdraw_to_lockup(&mut self, lockup_account_id: String, amount: U128) -> Promise;
}
/// Interface for the contract itself.
#[ext_contract(ext_self)]
pub trait SelfContract {
    /// A callback to check the result of the staking action.
    /// In case the stake failed, this callback rollbacks changes
    fn after_stake_for_lockup(&mut self, account_id: AccountId, deposited_amount: U128);
    fn after_metapool_withdraw_to_lockup(&mut self, account_id: AccountId, amount: U128);
    fn after_unstake_shares(&mut self, account_id: AccountId, num_shares: U128);
}

const NOT_SUPPORTED_PLEASE_USE_DEPOSIT_AND_STAKE: &str =
    "not supported, please use deposit_and_stake";

#[near_bindgen]
impl StakingContract {
    // =====================
    // == DEPOSIT & STAKE ==
    // =====================

    // Note: In the reference contract near-core/staking-pool, depositing and staking
    // can be performed separately or in a single call:
    // There are functions called: `deposit` then `stake` and `stake_all`, and the composed `deposit_and_stake`.
    // To increase safety and simplicity, we only support the simpler `deposit_and_stake` method.
    // By removing the concept of "locally deposited balance" the contract becomes simpler and thus more secure.
    // Note 1: All unstake and withdraw functions are supported.
    // Note 2: The standard wallet uses deposit_and_stake when dealing with lockup accounts
    #[payable]
    pub fn deposit(&mut self) {
        panic!("{}", NOT_SUPPORTED_PLEASE_USE_DEPOSIT_AND_STAKE);
    }

    /// Stakes all available unstaked balance from the inner account of the predecessor.
    pub fn stake_all(&mut self) -> Promise {
        panic!("{}", NOT_SUPPORTED_PLEASE_USE_DEPOSIT_AND_STAKE);
    }

    /// Stakes the given amount from the inner account of the predecessor.
    /// The inner account should have enough unstaked balance.
    #[allow(unused_variables)]
    pub fn stake(&mut self, amount: U128) -> Promise {
        panic!("{}", NOT_SUPPORTED_PLEASE_USE_DEPOSIT_AND_STAKE);
    }

    /// Deposits the attached amount into the inner account of the predecessor and stakes it.
    /// Note: The foundation-s near-core/lockup-contract USES 50GAS for this call
    #[payable]
    pub fn deposit_and_stake(&mut self) -> Promise {
        let account_id = env::predecessor_account_id();
        let amount = env::attached_deposit();

        // we're managing lockup.accounts, keep a sane minimum
        assert!(amount >= 10 * ONE_NEAR, "minimum deposit amount is 10 NEAR");

        // avoiding re-entry
        self.set_account_busy_flag_or_panic(&account_id);
        // call meta pool to stake
        ext_metapool::stake_for_lockup(
            account_id.to_string(),
            //---
            self.meta_pool_contract_id.clone(),
            amount, // send the NEAR
            Gas(META_POOL_DEPOSIT_AND_STAKE_GAS),
            )
        .then(ext_self::after_stake_for_lockup(
            account_id,
            amount.into(),
            //---
            env::current_account_id(),
            0,
            Gas(AFTER_STAKE_FOR_LOCKUP_GAS),
        ))
    }
    #[private]
    // continues after previous fn
    pub fn after_stake_for_lockup(&mut self, account_id: AccountId, deposited_amount: U128) {
        // WARN: This is a callback after-cross-contract-call method
        // busy locks must be saved false in the state, this method SHOULD NOT PANIC
        // SO DO NOT USE `#[callback]num_shares:U128` arguments, decode the return value manually

        // Check promise result and det the received_nears from the promise result.
        match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),

            PromiseResult::Successful(value) => {
                if let Ok(num_shares) = near_sdk::serde_json::from_slice::<U128>(&value) {
                    let num_shares = num_shares.0;
                    log!(
                        "@{} deposit_and_stake {} received {} shares",
                        account_id,
                        deposited_amount.0,
                        num_shares
                    );
                    // register shares received
                    let mut account = self.internal_get_account(&account_id);
                    account.busy = false;
                    account.stake_shares += num_shares;
                    self.internal_save_account(&account_id, &account);
                    // update also contract total
                    self.total_stake_shares += num_shares;
                } else {
                    // promise ok but no result? -- should not happen
                    log!("UNEXPECTED ERROR: promise ok but no result!",);
                }
            }

            PromiseResult::Failed => {
                // stake at meta pool failed, ROLLBACK
                self.clear_busy_flag(&account_id);
                // return NEARs to the lockup-account
                Promise::new(account_id).transfer(deposited_amount.0);
            }
        };
    }

    // =============
    // == UNSTAKE ==
    // =============

    /// Unstakes all staked balance from the inner account of the predecessor.
    /// The new total unstaked balance will be available for withdrawal in x epochs.
    pub fn unstake_all(&mut self) -> Promise {
        let account_id = env::predecessor_account_id();
        let account = self.internal_get_account(&account_id);
        self.inner_unstake_shares(&account_id, account.stake_shares)
    }

    /// Unstakes the given amount (in NEARs) from the inner account of the predecessor.
    /// The inner account should have enough staked balance.
    /// The new total unstaked balance will be available for withdrawal in four epochs.
    /// given that the share price increases with staking rewards, it is possible that final amount
    /// withdrawn could be higher because of the inclusion of new staking rewards
    /// (the amount could only be higher, not lower)
    pub fn unstake(&mut self, amount: U128) -> Promise {
        let amount: Balance = amount.into();
        let shares = mul_div(amount, ONE_NEAR, self.share_near_price);
        self.inner_unstake_shares(&env::predecessor_account_id(), shares)
    }

    fn inner_unstake_shares(&mut self, account_id: &AccountId, num_shares: u128) -> Promise {
        assert!(num_shares > 0, "Unstaking share amount should be positive");

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

        // avoid re-entry
        self.set_account_busy_flag_or_panic(&account_id);
        // call meta pool
        ext_metapool::unstake_from_lockup_shares(
            account_id.to_string(),
            num_shares.into(),
            //---
            self.meta_pool_contract_id.clone(),
            0,
            Gas(META_POOL_UNSTAKE_SHARES_GAS),
            )
        .then(ext_self::after_unstake_shares(
            account_id.clone(),
            num_shares.into(),
            //---
            env::current_account_id(),
            0,
            Gas(AFTER_UNSTAKE_SHARES_GAS),
        ))
    }
    #[private]
    // continues after previous fn
    pub fn after_unstake_shares(&mut self, account_id: AccountId, num_shares: U128) {
        // WARN: This is a callback after-cross-contract-call method
        // busy locks must be saved false in the state, this method SHOULD NOT PANIC
        // SO DO NOT USE `#[callback]received_nears:U128` arguments, decode the return value manually

        // convert to u128
        let num_shares = num_shares.0;

        // Check promise result and subtract the received_nears from the promise result.
        match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),

            PromiseResult::Successful(value) => {
                if let Ok((unstaked_nears, unstaked_available_epoch_height)) =
                    near_sdk::serde_json::from_slice::<(U128, U64)>(&value)
                {
                    let unstaked_nears = unstaked_nears.0;
                    // register the successful unstake share
                    let mut account = self.internal_get_account(&account_id);
                    account.busy = false;
                    account.stake_shares -= num_shares;
                    account.unstaked_in_metapool += unstaked_nears;
                    account.unstaked_available_epoch_height = unstaked_available_epoch_height.0;
                    self.internal_save_account(&account_id, &account);
                    // update contract totals
                    self.total_stake_shares -= num_shares;
                    log!(
                        "unstake shares at meta pool OK! account:{}, shares:{}, unstaked_nears:{}. Contract shares:{} ",
                        account_id,
                        num_shares,
                        unstaked_nears,
                        self.total_stake_shares
                    );
                } else {
                    // promise ok but no result? -- should not happen
                    log!("UNEXPECTED ERROR: promise ok but no result!",);
                }
            }

            PromiseResult::Failed => {
                // unstake shares at meta pool failed!
                self.clear_busy_flag(&account_id);
                log!(
                    "ERR: unstake shares at meta pool failed! account {}, shares {}",
                    account_id,
                    num_shares
                );
            }
        };
    }

    // ==============
    // == WITHDRAW ==
    // ==============

    /// Withdraws the entire unstaked balance from the predecessor account.
    /// It's only allowed if the `unstake` action was not performed in the four most recent epochs.
    pub fn withdraw_all(&mut self) -> Promise {
        let account_id = env::predecessor_account_id();
        let account = self.internal_get_account(&account_id);
        self.perform_withdraw(&account_id, account.unstaked_in_metapool)
    }

    /// Withdraws the non staked balance for given account.
    /// It's only allowed if the `unstake` action was not performed in the four most recent epochs.
    pub fn withdraw(&mut self, amount: U128) -> Promise {
        self.perform_withdraw(&env::predecessor_account_id(), amount.into())
    }

    fn perform_withdraw(&mut self, account_id: &AccountId, amount: Balance) -> Promise {
        assert!(amount > 0, "Withdrawal amount should be positive");
        let account = self.internal_get_account(&account_id);
        // the user has enough balance?
        assert!(
            account.unstaked_in_metapool >= amount,
            "Not enough unstaked balance to withdraw"
        );

        // Note: the reference contract is near-core/staking-contract from the NEAR foundation.
        // In that contract, asking for unstake locks all funds, including any funds deposited but not staked yet.
        // https://github.com/near/core-contracts/blob/3f3170fce91ff4d8c6ee9d15683f2d4dfe1275cf/staking-pool/src/internal.rs#L42
        // Here we need to replicate the same mechanics: reject withdrawals if env::epoch_height() < unstaked_available_epoch_height
        // even if there are funds in account.unstaked, in order to emulated the expected behavior set by near-core/staking-contract

        // make sure the wait period is over
        assert!(
            account.unstaked_available_epoch_height <= env::epoch_height(),
            "The unstaked balance is not yet available due to unstaking delay"
        );

        // avoiding re-entry
        self.set_account_busy_flag_or_panic(&account_id);
        // call metapool. The NEAR will be sent directly to the lockup account
        ext_metapool::withdraw_to_lockup(
            account_id.to_string(),
            amount.into(),
            //--
            self.meta_pool_contract_id.clone(),
            0,
            Gas(META_POOL_WITHDRAW_GAS),
            )
        .then(ext_self::after_metapool_withdraw_to_lockup(
            account_id.clone(),
            amount.into(),
            //--
            env::current_account_id(),
            0,
            Gas(AFTER_WITHDRAW_GAS),
        ))
    }
    #[private]
    // continues after previous fn
    pub fn after_metapool_withdraw_to_lockup(&mut self, account_id: AccountId, amount: U128) {
        // WARN: This is a callback after-cross-contract-call method
        // busy locks must be saved false in the state, this method SHOULD NOT PANIC
        let amount = amount.0;
        if is_promise_success() {
            // withdraw success
            log!(
                "withdraw from meta pool to {} for {} yNEAR succeeded",
                account_id,
                amount,
            );
            // the amount was sent by meta-pool to the lockup account
            let mut account = self.internal_get_account(&account_id);
            account.busy = false;
            account.unstaked_in_metapool = account.unstaked_in_metapool.saturating_sub(amount);
            // save account
            self.internal_save_account(&account_id, &account);
        } else {
            // failed!
            self.clear_busy_flag(&account_id);
            // withdraw at meta pool failed, but we can not panic here, just log
            log!(
                "ERROR! @{} asking for METAPOOL withdraw {} FAILED",
                account_id,
                amount,
            );
        }
    }
}
