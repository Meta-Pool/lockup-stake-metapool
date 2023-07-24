use near_sdk::{env::is_valid_account_id, assert_one_yocto};

use crate::*;

///*******************/
///* Owner's methods */
///*******************/
#[near_bindgen]
impl StakingContract {

    /// Changes contract owner. Must be called by current owner.
    #[payable]
    pub fn set_owner_id(&mut self, new_owner_id: &AccountId) {
        assert_one_yocto();
        assert!(is_valid_account_id(&new_owner_id.as_bytes()));
        assert_eq!(
            self.owner_id,
            env::predecessor_account_id(),
            "MUST BE OWNER TO SET OWNER"
        );
        self.owner_id = new_owner_id.clone();
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
