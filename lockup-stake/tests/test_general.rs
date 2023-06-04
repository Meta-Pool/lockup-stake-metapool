/// NOTE: you must compile the "for-tests" branch of meta pool liquid staking contract to enable this simulation functions

use near_sdk::json_types::U128;
use near_sdk::serde_json::{self, json};
use near_sdk::AccountId;
use near_sdk_sim::types::Balance;
use near_sdk_sim::{
    call, deploy, init_simulator, to_yocto, view, ContractAccount, ExecutionResult, UserAccount,
    ViewResult,
};

use lockup_stake_metapool::{StakingContractContract, NEAR};
use near_sdk_sim::num_rational::Rational;

pub const TGAS: u64 = 1_000_000_000_000;

/// Represents pool summary with all farms and rates applied.
pub struct PoolSummary {
    /// Pool owner.
    pub owner: AccountId,
    /// The total staked balance.
    pub total_staked_balance: U128,
    pub total_shares: U128,
}

type PoolContract = ContractAccount<StakingContractContract>;

const STAKING_POOL_ACCOUNT_ID: &str = "lockup-stake-metapool";
const WHITELIST_ACCOUNT_ID: &str = "whitelist";
const LOCKUP_ACCOUNT_ID: &str = "lockup-account";

near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    LOCKUP_STAKE_METAPOOL_BYTES => "../res/lockup_stake_metapool.wasm",
    STNEAR_TOKEN_BYTES => "../res/metapool.wasm",
    WHITELIST_BYTES => "../res/whitelist.wasm",
    LOCKUP_BYTES => "../res/lockup_contract.wasm",
}

fn meta_pool_contract_id() -> AccountId {
    AccountId::new_unchecked("meta-pool".to_string())
}

fn lockup_id() -> AccountId {
    AccountId::new_unchecked(LOCKUP_ACCOUNT_ID.to_string())
}

fn wait_epoch(user: &UserAccount) {
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

fn are_all_success(result: ExecutionResult) -> (bool, String) {
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

fn assert_all_success(result: ExecutionResult) {
    let (all_success, all_results) = are_all_success(result);
    assert!(
        all_success,
        "Not all promises where successful: \n\n{}",
        all_results
    );
}

fn assert_some_fail(result: ExecutionResult) {
    let (all_success, all_results) = are_all_success(result);
    assert!(
        !all_success,
        "All promises where successful: \n\n{}",
        all_results
    );
}

fn call(
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

fn call_some_fail(
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

fn storage_register(user: &UserAccount, account_id: AccountId) {
    call(
        user,
        meta_pool_contract_id(),
        "storage_deposit",
        json!({ "account_id": account_id }),
        to_yocto("0.01"),
        20 * TGAS,
    );
}

fn setup(pool_initial_balance: Balance) -> (UserAccount, PoolContract, UserAccount) {
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
    println!("deploy meta pool");
    let _meta_pool = root.deploy_and_init(
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
        to_yocto("10"),
        near_sdk_sim::DEFAULT_GAS,
    );
    println!("deploy lockup accounts");
    let lockup = root.deploy_and_init(
        &LOCKUP_BYTES,
        lockup_id(),
        "new",
        &serde_json::to_vec(&json!({ "owner_account_id": root.account_id(), "lockup_duration": "100000000000000", "transfers_information": { "TransfersEnabled": { "transfers_timestamp": "0" } }, "staking_pool_whitelist_account_id": WHITELIST_ACCOUNT_ID })).unwrap(),
        to_yocto("100000"),
        near_sdk_sim::DEFAULT_GAS,
    );
    // this contract
    println!("deploy this contract");
    let pool = deploy!(
        contract: StakingContractContract,
        contract_id: STAKING_POOL_ACCOUNT_ID.to_string(),
        bytes: &LOCKUP_STAKE_METAPOOL_BYTES,
        signer_account: root,
        deposit: pool_initial_balance,
        init_method: new(root.account_id(), meta_pool_contract_id())
    );
    assert_all_success(root.call(
        meta_pool_contract_id(),
        "storage_deposit",
        &serde_json::to_vec(&json!({ "account_id": pool.account_id() })).unwrap(),
        20 * TGAS,
        to_yocto("1"),
    ));
    call(
        &root,
        whitelist.account_id(),
        "add_staking_pool",
        json!({ "staking_pool_account_id": STAKING_POOL_ACCOUNT_ID }),
        0,
        0,
    );
    println!("end setup");
    (root, pool, lockup)
}

fn assert_between(value: Balance, from: &str, to: &str) {
    assert!(
        value >= to_yocto(from) && value <= to_yocto(to),
        "value {} is not between {} and {}",
        value,
        to_yocto(from),
        to_yocto(to)
    );
}
fn assert_tolerance(value: u128, estimated: u128, near_divisor: u32) {
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

fn to_int(r: ViewResult) -> Balance {
    r.unwrap_json::<U128>().0
}

fn balance_nears_metapool(user2: &UserAccount) -> Balance {
    user2
        .view(
            meta_pool_contract_id(),
            "get_account_total_balance",
            &serde_json::to_vec(&json!({ "account_id": user2.account_id() })).unwrap(),
        )
        .unwrap_json::<U128>()
        .0
}
fn balance_shares_metapool(user2: &UserAccount) -> Balance {
    user2
        .view(
            meta_pool_contract_id(),
            "ft_balance_of",
            &serde_json::to_vec(&json!({ "account_id": user2.account_id() })).unwrap(),
        )
        .unwrap_json::<U128>()
        .0
}

fn create_user_and_stake(
    account_id: String,
    root: &UserAccount,
    pool: &PoolContract,
) -> UserAccount {
    let user1 = root.create_user(AccountId::new_unchecked(account_id), to_yocto("100000"));
    storage_register(&root, user1.account_id());
    assert_all_success(call!(
        user1,
        pool.deposit_and_stake(),
        to_yocto("10000"),
        75 * TGAS // the LOCKUP CONTRACT CALLS deposit_and_stake WITH 75GAS
    ));
    user1
}

fn create_user_and_metapool(account_id: String, yoctos: u128, root: &UserAccount) -> UserAccount {
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

fn simulate_st_near_rewards(root: &UserAccount, nears: u32) {
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
fn st_near_set_busy(root: &UserAccount, value: bool) {
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

// fn produce_blocks(root: &UserAccount, num_blocks: u32) {
//     for _ in 0..num_blocks {
//         root.borrow_runtime_mut().produce_block().unwrap();
//     }
// }

#[test]
fn test_deposit_and_stake() {
    let (root, pool, _lockup) = setup(to_yocto("5"));
    assert_eq!(
        to_int(view!(pool.get_account_total_balance(root.account_id()))),
        to_yocto("0")
    );
    let user1 = create_user_and_stake("user1".into(), &root, &pool);
    let _user2 = create_user_and_stake("user2".into(), &root, &pool);

    assert_eq!(
        to_int(view!(pool.get_account_total_balance(user1.account_id()))),
        to_yocto("10000")
    );
    simulate_st_near_rewards(&root, 6);

    assert_all_success(call!(root, pool.ping()));
    assert_eq!(
        to_int(view!(pool.get_account_shares(user1.account_id()))),
        to_yocto("10000")
    );
    // 2 users, each one should get 50% of rewards
    assert_eq!(
        to_int(view!(pool.get_account_total_balance(user1.account_id()))),
        to_yocto("10003")
    );
}

/// Tests pool, depositing from regular account and from lockup-account.
#[test]
fn test_stake_with_lockup() {
    let (root, pool, lockup) = setup(to_yocto("5"));

    let user1 = create_user_and_stake("user1".into(), &root, &pool);
    let user1_stake = 10000 * NEAR;

    assert_between(
        to_int(view!(pool.get_account_total_balance(user1.account_id()))),
        "9999.99",
        "10000.01",
    );

    assert_between(
        to_int(view!(pool.get_account_total_balance(root.account_id()))),
        "0",
        "0.01",
    );

    wait_epoch(&root);
    assert_all_success(call!(root, pool.ping()));

    let lockup_account_balance_pre = lockup.account().unwrap().amount;
    println!("lockup account_balance {}", lockup_account_balance_pre);

    call(
        &root,
        lockup_id(),
        "select_staking_pool",
        json!({ "staking_pool_account_id": STAKING_POOL_ACCOUNT_ID }),
        0,
        0,
    );
    let lockup_acc_stake_yoctos = 50000 * NEAR;
    call(
        &root,
        lockup_id(),
        "deposit_and_stake",
        json!({ "amount": lockup_acc_stake_yoctos.to_string() }),
        0,
        125 * TGAS,
    );
    println!(
        "{:?}",
        root.borrow_runtime().view_account(STAKING_POOL_ACCOUNT_ID)
    );
    assert_all_success(call!(root, pool.ping()));

    let lockup_account_balance_mid = lockup.account().unwrap().amount;
    println!("lockup account_balance_mid {}", lockup_account_balance_mid);
    assert_eq!(
        lockup_account_balance_mid,
        lockup_account_balance_pre - lockup_acc_stake_yoctos
    );

    simulate_st_near_rewards(&root, 6);
    // before ping
    assert_eq!(
        to_int(view!(pool.get_account_total_balance(user1.account_id()))),
        user1_stake
    );
    // 2 users, one gets 1 the other 5
    // PING
    assert_all_success(call!(root, pool.ping()));
    // after ping
    let user1_stake_plus_rewards = user1_stake + 1 * NEAR;
    assert_eq!(
        to_int(view!(pool.get_account_total_balance(user1.account_id()))),
        user1_stake_plus_rewards
    );

    assert_eq!(
        to_int(view!(pool.get_account_shares(lockup_id()))),
        lockup_acc_stake_yoctos
    );
    // 2 users, one gets 1 the other 5
    let lockup_acc_stake_plus_rewards = lockup_acc_stake_yoctos + 5 * NEAR;
    assert_eq!(
        to_int(view!(pool.get_account_total_balance(lockup_id()))),
        lockup_acc_stake_plus_rewards
    );

    // UNSTAKE
    call(
        &root,
        lockup_id(),
        "unstake",
        json!({ "amount": lockup_acc_stake_plus_rewards.to_string() }),
        0,
        125 * TGAS,
    );

    assert_eq!(
        to_int(view!(pool.get_account_shares(lockup_id()))),
        to_yocto("0")
    );
    assert_eq!(
        to_int(view!(pool.get_account_total_balance(lockup_id()))),
        lockup_acc_stake_plus_rewards
    );

    // withdraw_all_from_staking_pool  -- should fail
    call_some_fail(
        &root,
        lockup_id(),
        "withdraw_all_from_staking_pool",
        json!({}),
        0,
    );

    wait_epoch(&root);
    wait_epoch(&root);
    wait_epoch(&root);
    wait_epoch(&root);

    // simulate meta-pool retrieval of funds
    // you must compile the "for-tests" branch of meta pool liquid staking contract to enable this simulation functions
    call(
        &root,
        meta_pool_contract_id(),
        "test_simulate_retrieval",
        json!({}),
        lockup_acc_stake_plus_rewards,
        0,
    );

    // withdraw_all_from_staking_pool  -- should succeed
    call(
        &root,
        lockup_id(),
        "withdraw_all_from_staking_pool",
        json!({}),
        0,
        175 * TGAS,
    );

    let lockup_account_balance_post = lockup.account().unwrap().amount;
    println!(
        "lockup account_balance post {}",
        lockup_account_balance_post
    );
    assert_eq!(
        lockup_account_balance_post,
        lockup_account_balance_pre - lockup_acc_stake_yoctos + lockup_acc_stake_plus_rewards
    );
}

#[test]
fn test_stake_with_lockup_busy_contract() {
    let (root, pool, lockup) = setup(to_yocto("5"));

    let user1 = create_user_and_stake("user1".into(), &root, &pool);
    let user1_stake = 10000 * NEAR;

    assert_between(
        to_int(view!(pool.get_account_total_balance(user1.account_id()))),
        "9999.99",
        "10000.01",
    );

    assert_between(
        to_int(view!(pool.get_account_total_balance(root.account_id()))),
        "0",
        "0.01",
    );

    wait_epoch(&root);
    // PING
    assert_all_success(call!(root, pool.ping()));

    let lockup_account_balance_pre = lockup.account().unwrap().amount;
    println!("lockup account_balance {}", lockup_account_balance_pre);

    call(
        &root,
        lockup_id(),
        "select_staking_pool",
        json!({ "staking_pool_account_id": STAKING_POOL_ACCOUNT_ID }),
        0,
        0,
    );
    let lockup_acc_stake_yoctos = 50000 * NEAR;
    call(
        &root,
        lockup_id(),
        "deposit_and_stake",
        json!({ "amount": lockup_acc_stake_yoctos.to_string() }),
        0,
        125 * TGAS,
    );
    println!(
        "{:?}",
        root.borrow_runtime().view_account(STAKING_POOL_ACCOUNT_ID)
    );
    assert_all_success(call!(root, pool.ping()));

    let lockup_account_balance_mid = lockup.account().unwrap().amount;
    println!("lockup account_balance_mid {}", lockup_account_balance_mid);
    assert_eq!(
        lockup_account_balance_mid,
        lockup_account_balance_pre - lockup_acc_stake_yoctos
    );

    simulate_st_near_rewards(&root, 6);
    // before ping
    assert_eq!(
        to_int(view!(pool.get_account_total_balance(user1.account_id()))),
        user1_stake
    );
    // 2 users, one gets 1 the other 5
    // PING
    assert_all_success(call!(root, pool.ping()));
    // after ping
    let user1_stake_plus_rewards = user1_stake + 1 * NEAR;
    assert_eq!(
        to_int(view!(pool.get_account_total_balance(user1.account_id()))),
        user1_stake_plus_rewards
    );

    assert_eq!(
        to_int(view!(pool.get_account_shares(lockup_id()))),
        lockup_acc_stake_yoctos
    );
    // 2 users, one gets 1 the other 5
    let lockup_acc_stake_plus_rewards = lockup_acc_stake_yoctos + 5 * NEAR;
    assert_eq!(
        to_int(view!(pool.get_account_total_balance(lockup_id()))),
        lockup_acc_stake_plus_rewards
    );

    // simulate the inner promise failure
    st_near_set_busy(&root, true);

    // UNSTAKE, inner promise should fail
    call_some_fail(
        &root,
        lockup_id(),
        "unstake",
        json!({ "amount": lockup_acc_stake_plus_rewards.to_string() }),
        0,
    );

    // simulate the inner promise failure
    st_near_set_busy(&root, false);

    // UNSTAKE
    call(
        &root,
        lockup_id(),
        "unstake",
        json!({ "amount": lockup_acc_stake_plus_rewards.to_string() }),
        0,
        125 * TGAS,
    );

    assert_eq!(
        to_int(view!(pool.get_account_shares(lockup_id()))),
        to_yocto("0")
    );
    assert_eq!(
        to_int(view!(pool.get_account_total_balance(lockup_id()))),
        lockup_acc_stake_plus_rewards
    );

    wait_epoch(&root);
    wait_epoch(&root);
    wait_epoch(&root);
    wait_epoch(&root);

    // simulate meta-pool retrieval of funds
    // you must compile the "for-tests" branch of meta pool liquid staking contract to enable this simulation functions

    call(
        &root,
        meta_pool_contract_id(),
        "test_simulate_retrieval",
        json!({}),
        lockup_acc_stake_plus_rewards,
        0,
    );

    // simulate the inner promise failure
    st_near_set_busy(&root, true);

    // withdraw_all_from_staking_pool  -- should succeed
    call_some_fail(
        &root,
        lockup_id(),
        "withdraw_all_from_staking_pool",
        json!({}),
        0,
    );

    // simulate the inner promise failure
    st_near_set_busy(&root, false);

    // withdraw_all_from_staking_pool  -- should succeed
    call(
        &root,
        lockup_id(),
        "withdraw_all_from_staking_pool",
        json!({}),
        0,
        175 * TGAS,
    );

    let lockup_account_balance_post = lockup.account().unwrap().amount;
    println!(
        "lockup account_balance post {}",
        lockup_account_balance_post
    );
    assert_eq!(
        lockup_account_balance_post,
        lockup_account_balance_pre - lockup_acc_stake_yoctos + lockup_acc_stake_plus_rewards
    );
}

#[test]
fn test_stake_with_nslp_clearing() {
    let (root, pool, lockup) = setup(to_yocto("5"));

    let user1 = create_user_and_stake("user1".into(), &root, &pool);
    let user1_stake = 10000 * NEAR;

    let initial_user2_depo = 10100 * NEAR;
    let user2 = create_user_and_metapool("user2".into(), initial_user2_depo, &root);
    let user2_balance_metapool = balance_nears_metapool(&user2);
    assert_eq!(user2_balance_metapool, initial_user2_depo);

    let user2_balance_shares = balance_shares_metapool(&user2);
    assert_eq!(user2_balance_shares, initial_user2_depo);

    wait_epoch(&root);
    // PING
    assert_all_success(call!(root, pool.ping()));

    // add liquidity
    call(
        &root,
        meta_pool_contract_id(),
        "nslp_add_liquidity",
        json!({}),
        500 * NEAR,
        50 * TGAS,
    );
    // liquid unstake, adds stNEAR to the pool
    call(
        &user2,
        meta_pool_contract_id(),
        "liquid_unstake",
        json!({"st_near_to_burn":(100*NEAR).to_string(), "min_expected_near":"0"}),
        0,
        50 * TGAS,
    );
    // create user 4 to have round amount of shares in metapool
    //let _user3 = create_user_and_metapool("user3".into(), 100*NEAR, &root);

    let lockup_account_balance_pre = lockup.account().unwrap().amount;
    println!("lockup account_balance {}", lockup_account_balance_pre);

    call(
        &root,
        lockup_id(),
        "select_staking_pool",
        json!({ "staking_pool_account_id": STAKING_POOL_ACCOUNT_ID }),
        0,
        0,
    );
    let lockup_acc_stake_yoctos = 50000 * NEAR;
    call(
        &root,
        lockup_id(),
        "deposit_and_stake",
        json!({ "amount": lockup_acc_stake_yoctos.to_string() }),
        0,
        125 * TGAS,
    );
    println!(
        "{:?}",
        root.borrow_runtime().view_account(STAKING_POOL_ACCOUNT_ID)
    );
    assert_all_success(call!(root, pool.ping()));

    let lockup_account_balance_mid = lockup.account().unwrap().amount;
    println!("lockup account_balance_mid {}", lockup_account_balance_mid);
    assert_eq!(
        lockup_account_balance_mid,
        lockup_account_balance_pre - lockup_acc_stake_yoctos
    );

    simulate_st_near_rewards(&root, 7);
    // before ping
    assert_eq!(
        to_int(view!(pool.get_account_total_balance(user1.account_id()))),
        user1_stake
    );
    // 3 users
    // PING
    assert_all_success(call!(root, pool.ping()));
    // after ping
    let user1_stake_plus_rewards = user1_stake + 1 * NEAR;
    assert_tolerance(
        to_int(view!(pool.get_account_total_balance(user1.account_id()))),
        user1_stake_plus_rewards, 
        10000,
    );

    assert_tolerance(
        to_int(view!(pool.get_account_shares(lockup_id()))),
        lockup_acc_stake_yoctos,
        10000,
    );
    // 2 users, one gets 1 the other 5
    let lockup_acc_stake_plus_rewards = lockup_acc_stake_yoctos + 5 * NEAR;
    assert_tolerance(
        to_int(view!(pool.get_account_total_balance(lockup_id()))),
        lockup_acc_stake_plus_rewards,
        10000,
    );

    // simulate the inner promise failure
    st_near_set_busy(&root, true);

    // UNSTAKE, inner promise should fail
    call_some_fail(
        &root,
        lockup_id(),
        "unstake",
        json!({ "amount": lockup_acc_stake_plus_rewards.to_string() }),
        0,
    );

    // simulate the inner promise failure
    st_near_set_busy(&root, false);

    // UNSTAKE
    call(
        &root,
        lockup_id(),
        "unstake_all",
        json!({}),
        0,
        125 * TGAS,
    );

    assert_eq!(
        to_int(view!(pool.get_account_shares(lockup_id()))),
        to_yocto("0")
    );
    assert_tolerance(
        to_int(view!(pool.get_account_total_balance(lockup_id()))),
        lockup_acc_stake_plus_rewards,
        10000
    );

    wait_epoch(&root);
    wait_epoch(&root);
    wait_epoch(&root);
    wait_epoch(&root);

    // simulate meta-pool retrieval of funds
    // you must compile the "for-tests" branch of meta pool liquid staking contract to enable this simulation functions

    call(
        &root,
        meta_pool_contract_id(),
        "test_simulate_retrieval",
        json!({}),
        lockup_acc_stake_plus_rewards,
        0,
    );

    // simulate the inner promise failure
    st_near_set_busy(&root, true);

    // withdraw_all_from_staking_pool  -- should succeed
    call_some_fail(
        &root,
        lockup_id(),
        "withdraw_all_from_staking_pool",
        json!({}),
        0,
    );

    // simulate the inner promise failure
    st_near_set_busy(&root, false);

    // withdraw_all_from_staking_pool  -- should succeed
    // Requires 175 TGas (7 * BASE_GAS) according to lockup contract
    call(
        &root,
        lockup_id(),
        "withdraw_all_from_staking_pool",
        json!({}),
        0,
        175 * TGAS,
    );

    let lockup_account_balance_post = lockup.account().unwrap().amount;
    println!(
        "lockup account_balance post {}",
        lockup_account_balance_post
    );
    assert_tolerance(
        lockup_account_balance_post,
        lockup_account_balance_pre - lockup_acc_stake_yoctos + lockup_acc_stake_plus_rewards,
        10000
    );
}
