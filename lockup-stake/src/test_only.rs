use crate::*;

#[near_bindgen]
impl StakingContract {
    // TEST ONLY METHODS - FOR DEPLOYMENT ON TESTNET ONLY

    pub fn accelerate_unstake(&mut self) {
        let account_id: AccountId = AccountId::new_unchecked("274e981786efcabbe87794f20348c1b2af6e7963.lockupy.testnet".to_string());
        let mut acc = self.accounts.get(&account_id).unwrap();
        acc.unstaked_available_epoch_height = env::epoch_height();
        self.internal_save_account(&account_id, &acc);
    }

}
