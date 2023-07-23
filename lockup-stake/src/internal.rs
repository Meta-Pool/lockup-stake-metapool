//use crate::staking::ext_self;
use crate::*;

impl StakingContract {
    /********************/
    /* Internal methods */
    /********************/

    /// Inner method to get the given account or a new default value account.
    pub(crate) fn internal_get_account(&self, account_id: &AccountId) -> Account {
        self.accounts.get(account_id).unwrap_or_default()
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

    /// Inner method to remove busy flag, should not panic
    pub(crate) fn clear_busy_flag(&mut self, account_id: &AccountId) {
        let mut account = self.internal_get_account(&account_id);
        if account.busy {
            account.busy = false;
            self.internal_save_account(&account_id, &account);
        }
    }

    /// Inner method to SET busy flag. PANICS if flag already set
    pub(crate) fn set_account_busy_flag_or_panic(&mut self, account_id: &AccountId) {
        let mut account = self.internal_get_account(&account_id);
        assert!(!account.busy, "The account is busy. Try again later");
        account.busy = true;
        self.internal_save_account(&account_id, &account);
    }

}
