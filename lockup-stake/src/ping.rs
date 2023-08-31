use near_sdk::env;

use near_sdk::ext_contract;
use crate::*;
use crate::utils::TGAS;

// Note: looks like that on promises, near core adds 5 extra TGAS on each call
pub const GET_FUNCTION_GAS: u64 = 8 * TGAS;
pub const AFTER_GET_FUNCTION_GAS: u64 = 4 * TGAS;

/// Interface for Meta Pool
#[ext_contract(ext_metapool)]
trait MetaPool {
    fn get_st_near_price(&self) -> U128;
    fn get_reward_fee_bp(&self) -> u16;
}
/// Interface for the contract itself.
#[ext_contract(ext_self)]
pub trait ThisContract {
    // callbacks to receive the result of view function
    fn after_get_st_near_price(&self, #[callback] st_near_price: U128);
    fn after_get_reward_fee_bp(&self, #[callback] bp: u16);
}

#[near_bindgen]
impl StakingContract {

    /// gather info from meta pool, 
    /// st_near_price and current fee
    pub fn ping(&mut self) {
        // call meta pool
        // schedule 2 calls
        // 1. try get_st_near_price
        //log!("prepaid_gas {:?}, used_gas {:?}",env::prepaid_gas(), env::used_gas());
        ext_metapool::get_st_near_price(
            self.meta_pool_contract_id.clone(),
            0,
            Gas(GET_FUNCTION_GAS),
        )
        .then(ext_self::after_get_st_near_price(
            env::current_account_id(),
            0,
            Gas(AFTER_GET_FUNCTION_GAS),
        ));
        // 2. try get_reward_fee_bp
        //log!("prepaid_gas {:?}, used_gas {:?}",env::prepaid_gas(), env::used_gas());
        ext_metapool::get_reward_fee_bp(
            self.meta_pool_contract_id.clone(),
            0,
            Gas(GET_FUNCTION_GAS),
        )
        .then(ext_self::after_get_reward_fee_bp(
            env::current_account_id(),
            0,
            Gas(AFTER_GET_FUNCTION_GAS),
        ));
        //log!("prepaid_gas {:?}, used_gas {:?}",env::prepaid_gas(), env::used_gas());
    }
    #[private]
    // continues after previous fn
    pub fn after_get_st_near_price(&mut self, #[callback] st_near_price: U128) {
        // Note/Warn: because it uses #[callback], this fn does not execute if the promise fails
        self.share_near_price = st_near_price.0;
    }
    #[private]
    // continues after previous fn
    pub fn after_get_reward_fee_bp(&mut self, #[callback] bp: u16) {
        // Note/Warn: because it uses #[callback], this fn does not execute if the promise fails
        self.meta_pool_fee_bp = bp;
    }
}
