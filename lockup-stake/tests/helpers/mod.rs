use near_sdk::json_types::U128;
use near_sdk::serde_json::{self, json};
use near_sdk::AccountId;
use near_sdk_sim::types::Balance;
use near_sdk_sim::{
    call, deploy, init_simulator, to_yocto, ContractAccount, ExecutionResult, UserAccount,
    ViewResult,
};

use lockup_stake_metapool::{StakingContractContract, NEAR};
use near_sdk_sim::num_rational::Rational;

pub const TGAS: u64 = 1_000_000_000_000;

type LockupStakeContract = ContractAccount<StakingContractContract>;

pub const LOCKUP_STAKE_CONTRACT_ID: &str = "lockup.meta-pool.near";
pub const WHITELIST_ACCOUNT_ID: &str = "whitelist";
pub const TESTNET_ACCOUNT_ID: &str = "testnet";
pub const LOCKUPY_TESTNET_ACCOUNT_ID: &str = "lockupy.testnet";
pub const LOCKUP_ACCOUNT_ID: &str = "ab12345def.lockupy.testnet";

near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    LOCKUP_STAKE_METAPOOL_BYTES => "../res/lockup_stake_metapool.wasm",
    STNEAR_TOKEN_BYTES => "../res/metapool.wasm",
    WHITELIST_BYTES => "../res/whitelist.wasm",
    LOCKUP_BYTES => "../res/lockup_contract.wasm",
}

pub fn meta_pool_contract_id() -> AccountId {
    AccountId::new_unchecked("meta-pool.near".to_string())
}

pub fn lockup_account_id() -> AccountId {
    AccountId::new_unchecked(LOCKUP_ACCOUNT_ID.to_string())
}

pub fn wait_epoch(user: &UserAccount) {
    let epoch_height = user.borrow_runtime().cur_block.epoch_height;
    while user.borrow_runtime().cur_block.epoch_height == epoch_height {
        assert!(user.borrow_runtime_mut().produce_block().is_ok());
    }
    // sim framework doesn't provide block rewards.
    // model the block reward by sending more funds on the account.
    // user.transfer(
    //     AccountId::new_unchecked(STAKING_POOL_ACCOUNT_ID.to_string()),
    //     to_yocto("1000"),
    // );
    //simulate_st_near_rewards(&user, 4);
}

pub fn are_all_success(result: ExecutionResult) -> (bool, String) {
    let mut all_success = true;
    let mut all_results = String::new();
    for r in result.promise_results() {
        let x = r.expect("NO_RESULT");
        all_results = format!("{}\n{:?}", all_results, x);
        all_success &= x.is_ok();
    }
    for promise_result in result.promise_results() {
        println!("{:?}", promise_result.unwrap().outcome().logs);
    }
    (all_success, all_results)
}

pub fn assert_all_success(result: ExecutionResult) {
    let (all_success, all_results) = are_all_success(result);
    assert!(
        all_success,
        "Not all promises where successful: \n\n{}",
        all_results
    );
}

pub fn assert_some_fail(result: ExecutionResult) {
    let (all_success, all_results) = are_all_success(result);
    assert!(
        !all_success,
        "All promises where successful: \n\n{}",
        all_results
    );
}

pub fn call(
    user: &UserAccount,
    receiver_id: AccountId,
    method_name: &str,
    args: serde_json::Value,
    deposit: Balance,
    gas: u64,
) {
    println!("---------");
    println!(
        "call {} {} ({}) --accountId:{} --deposit:{}N {}Y --gas:{}TGAS",
        receiver_id,
        method_name,
        args,
        user.account_id,
        deposit / NEAR,
        deposit,
        gas / TGAS
    );
    assert_all_success(user.call(
        receiver_id,
        method_name,
        &serde_json::to_vec(&args).unwrap(),
        if gas == 0 {
            near_sdk_sim::DEFAULT_GAS
        } else {
            gas
        },
        deposit,
    ));
}

pub fn call_some_fail(
    user: &UserAccount,
    receiver_id: AccountId,
    method_name: &str,
    args: serde_json::Value,
    deposit: Balance,
) {
    println!(
        "call {}.{}({}) --accountId:{} --deposit:{}",
        receiver_id, method_name, args, user.account_id, deposit
    );
    assert_some_fail(user.call(
        receiver_id,
        method_name,
        &serde_json::to_vec(&args).unwrap(),
        near_sdk_sim::DEFAULT_GAS,
        deposit,
    ));
}

pub fn storage_register(user: &UserAccount, account_id: AccountId) {
    call(
        user,
        meta_pool_contract_id(),
        "storage_deposit",
        json!({ "account_id": account_id }),
        to_yocto("0.01"),
        20 * TGAS,
    );
}

pub fn setup() -> (UserAccount,UserAccount, LockupStakeContract, UserAccount) {
    let lockup_stake_initial_balance: Balance = 10 * NEAR;
    println!("start setup");
    let root = init_simulator(None);
    // Disable contract rewards.
    root.borrow_runtime_mut()
        .genesis
        .runtime_config
        .transaction_costs
        .burnt_gas_reward = Rational::new(0, 1);
    println!("deploy whitelist");
    let whitelist = root.deploy_and_init(
        &WHITELIST_BYTES,
        AccountId::new_unchecked(WHITELIST_ACCOUNT_ID.to_string()),
        "new",
        &serde_json::to_vec(&json!({ "foundation_account_id": root.account_id() })).unwrap(),
        to_yocto("10"),
        near_sdk_sim::DEFAULT_GAS,
    );
    println!("deploy meta lockup_stake");
    let near_user = root.create_user(AccountId::new_unchecked("near".to_string()), 500000 * NEAR);
    //let meta_pool_user = near_user.create_user(AccountId::new_unchecked("meta-pool".to_string()), 10 * NEAR);
    let meta_pool_contract_user = near_user.deploy_and_init(
        &STNEAR_TOKEN_BYTES,
        meta_pool_contract_id(),
        "new",
        &serde_json::to_vec(&json!({
            "owner_account_id": root.account_id(),
            "treasury_account_id": "treasury",
            "operator_account_id": "operator",
            "meta_token_account_id": "meta-token",
        }))
        .unwrap(),
        to_yocto("100"),
        near_sdk_sim::DEFAULT_GAS,
    );
    println!("create near account");
    let testnet_account = root.create_user(AccountId::new_unchecked(TESTNET_ACCOUNT_ID.to_string()), to_yocto("100000000"));
    let lockupy_testnet_account = testnet_account.create_user(AccountId::new_unchecked(LOCKUPY_TESTNET_ACCOUNT_ID.to_string()), to_yocto("1000000"));
    println!("deploy lockup accounts");
    let lockup = lockupy_testnet_account.deploy_and_init(
        &LOCKUP_BYTES,
        lockup_account_id(),
        "new",
        &serde_json::to_vec(&json!({ "owner_account_id": root.account_id(), "lockup_duration": "100000000000000", "transfers_information": { "TransfersEnabled": { "transfers_timestamp": "0" } }, "staking_pool_whitelist_account_id": WHITELIST_ACCOUNT_ID })).unwrap(),
        to_yocto("100000"),
        near_sdk_sim::DEFAULT_GAS,
    );
    // this contract
    println!("deploy this contract");
    let lockup_stake = deploy!(
        contract: StakingContractContract,
        contract_id: LOCKUP_STAKE_CONTRACT_ID.to_string(),
        bytes: &LOCKUP_STAKE_METAPOOL_BYTES,
        signer_account: meta_pool_contract_user,
        deposit: lockup_stake_initial_balance,
        init_method: new(root.account_id(), meta_pool_contract_id())
    );
    assert_all_success(root.call(
        meta_pool_contract_id(),
        "storage_deposit",
        &serde_json::to_vec(&json!({ "account_id": lockup_stake.account_id() })).unwrap(),
        20 * TGAS,
        to_yocto("1"),
    ));
    call(
        &root,
        whitelist.account_id(),
        "add_staking_pool",
        json!({ "staking_pool_account_id": LOCKUP_STAKE_CONTRACT_ID }),
        0,
        0,
    );
    println!("end setup");
    (root, lockupy_testnet_account, lockup_stake, lockup)
}

pub fn assert_between(value: Balance, from: &str, to: &str) {
    assert!(
        value >= to_yocto(from) && value <= to_yocto(to),
        "value {} is not between {} and {}",
        value,
        to_yocto(from),
        to_yocto(to)
    );
}
pub fn assert_tolerance(value: u128, estimated: u128, near_divisor: u32) {
    let tolerance = NEAR/near_divisor as u128;
    let from = estimated - tolerance;
    let to = estimated + tolerance;
    assert!(
        value >= from && value <= to,
        "value {} is not between {} and {}",
        value,
        from,
        to
    );
}

pub fn to_int(r: ViewResult) -> Balance {
    r.unwrap_json::<U128>().0
}

pub fn balance_nears_metapool(user2: &UserAccount) -> Balance {
    user2
        .view(
            meta_pool_contract_id(),
            "get_account_total_balance",
            &serde_json::to_vec(&json!({ "account_id": user2.account_id() })).unwrap(),
        )
        .unwrap_json::<U128>()
        .0
}
pub fn balance_shares_metapool(user2: &UserAccount) -> Balance {
    user2
        .view(
            meta_pool_contract_id(),
            "ft_balance_of",
            &serde_json::to_vec(&json!({ "account_id": user2.account_id() })).unwrap(),
        )
        .unwrap_json::<U128>()
        .0
}

pub fn create_user_and_stake(
    account_id: String,
    creator_account: &UserAccount,
    lockup_stake: &LockupStakeContract,
) -> UserAccount {
    let user1 = creator_account.create_user(AccountId::new_unchecked(account_id), to_yocto("100000"));
    storage_register(&creator_account, user1.account_id());
    println!("calling deposit_and_stake");
    assert_all_success(call!(
        user1,
        lockup_stake.deposit_and_stake(),
        to_yocto("10000"),
        75 * TGAS // the LOCKUP CONTRACT CALLS deposit_and_stake WITH 75GAS
    ));
    user1
}

pub fn create_user_and_metapool(account_id: String, yoctos: u128, root: &UserAccount) -> UserAccount {
    let new_user = root.create_user(AccountId::new_unchecked(account_id), to_yocto("100000"));
    storage_register(&root, new_user.account_id());
    call(
        &new_user,
        meta_pool_contract_id(),
        "deposit_and_stake",
        json!({}),
        yoctos,
        0,
    );
    new_user
}

pub fn simulate_st_near_rewards(root: &UserAccount, nears: u32) {
    wait_epoch(&root);
    // simulate some stNEAR price increase
    call(
        &root,
        meta_pool_contract_id(),
        "test_simulate_rewards",
        json!({}),
        to_yocto(&nears.to_string()),
        0,
    );
}
pub fn st_near_set_busy(root: &UserAccount, value: bool) {
    println!("----");
    println!("set busy {}", value);
    call(
        &root,
        meta_pool_contract_id(),
        "set_busy",
        json!({ "value": value }),
        1,
        0,
    );
}

// pub fn produce_blocks(root: &UserAccount, num_blocks: u32) {
//     for _ in 0..num_blocks {
//         root.borrow_runtime_mut().produce_block().unwrap();
//     }
// }
