
//use crate::staking::ext_self;
use crate::*;
use near_sdk::log;

impl StakingContract {
    /********************/
    /* Internal methods */
    /********************/

    pub(crate) fn internal_deposit(&mut self) -> (AccountId, Balance) {
        let account_id = env::predecessor_account_id();
        let mut account = self.internal_get_account(&account_id);
        let amount = env::attached_deposit();
        account.unstaked += amount;
        self.internal_save_account(&account_id, &account);

        log!(
            "@{} deposited {}. New unstaked balance is {}",
            account_id,
            amount,
            account.unstaked
        );
        (account_id, amount)
    }
    pub(crate) fn rollback_internal_deposit(&mut self, account_id: &AccountId, amount: Balance) {
        // rollbacks changes in prev fn
        // MUST NOT PANIC
        let mut account = self.internal_get_account(account_id);
        account.unstaked = account.unstaked.saturating_sub(amount);
        self.internal_save_account(&account_id, &account);

        log!(
            "UNDO @{} deposit {}. New unstaked balance is {}",
            account_id,
            amount,
            account.unstaked
        );
    }

    pub(crate) fn internal_register_staking(&mut self, amount: Balance) {
        assert!(amount > 0, "Staking amount should be positive");

        let account_id = env::predecessor_account_id();
        let mut account = self.internal_get_account(&account_id);

        assert!(
            account.unstaked >= amount,
            "Not enough unstaked balance to stake"
        );
        account.unstaked -= amount;
        self.internal_save_account(&account_id, &account);

        log!(
            "@{} staking {}. Total {} unstaked balance",
            account_id,
            amount,
            account.unstaked,
        );
    }
    pub(crate) fn rollback_internal_stake(&mut self, account_id: AccountId, amount: Balance) {
        // MUST NOT PANIC
        let mut account = self.internal_get_account(&account_id);
        account.unstaked += amount;
        self.internal_save_account(&account_id, &account);

        log!(
            "ROLLBACK @{} staking {}. Total {} unstaked balance",
            account_id,
            amount,
            account.unstaked,
        );
    }

    /// Inner method to get the given account or a new default value account.
    pub(crate) fn internal_get_account(&self, account_id: &AccountId) -> Account {
        let account = self.accounts.get(account_id).unwrap_or_default();
        account
    }

    /// Inner method to save the given account for a given account ID.
    /// If the account balances are 0, the account is deleted instead to release storage.
    pub(crate) fn internal_save_account(&mut self, account_id: &AccountId, account: &Account) {
        if account.is_empty() {
            self.accounts.remove(account_id);
        } else {
            self.accounts.insert(account_id, &account);
        }
    }

}
