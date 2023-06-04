use near_sdk::env::is_valid_account_id;

use crate::*;

///*******************/
///* Owner's methods */
///*******************/
#[near_bindgen]
impl StakingContract {

    /// Changes contract owner. Must be called by current owner.
    pub fn set_owner_id(&mut self, new_owner_id: &AccountId) {
        assert!(is_valid_account_id(&new_owner_id.as_bytes()));
        assert_eq!(
            self.owner_id,
            env::predecessor_account_id(),
            "MUST BE OWNER TO SET OWNER"
        );
        self.owner_id = new_owner_id.clone();
    }

    /// Owner's method.
    /// Pauses pool staking.
    pub fn pause_staking(&mut self) {
        self.assert_owner();
        assert!(!self.paused, "The staking is already paused");
        self.paused = true;
    }

    /// Owner's method.
    /// Resumes pool staking.
    pub fn resume_staking(&mut self) {
        self.assert_owner();
        assert!(self.paused, "The staking is not paused");
        self.paused = false;
    }

    /// Asserts that the method was called by the owner.
    pub(crate) fn assert_owner(&self) {
        assert_eq!(
            env::predecessor_account_id(),
            self.owner_id,
            "Can only be called by the owner"
        );
    }

}
