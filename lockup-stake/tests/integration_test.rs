// Note: you must compile the "with-tests-functions" branch of the liquid staking contract
// and copy the wasm in ../res, in order to enable test simulation functions for staking rewards

mod helpers;
use helpers::*;

use near_sdk::serde_json::json;
use near_sdk_sim::{call, to_yocto, view};

use lockup_stake_metapool::NEAR;

#[test]
fn test_deposit_and_stake() {
    let (root, lockup_stake, _lockup) = setup();
    assert_eq!(
        to_int(view!(
            lockup_stake.get_account_total_balance(root.account_id())
        )),
        to_yocto("0")
    );
    let user1 = create_user_and_stake("user1".into(), &root, &lockup_stake);
    let _user2 = create_user_and_stake("user2".into(), &root, &lockup_stake);

    assert_eq!(
        to_int(view!(
            lockup_stake.get_account_total_balance(user1.account_id())
        )),
        to_yocto("10000")
    );
    simulate_st_near_rewards(&root, 6);

    assert_all_success(call!(root, lockup_stake.ping()));
    assert_eq!(
        to_int(view!(lockup_stake.get_account_shares(user1.account_id()))),
        to_yocto("10000")
    );
    // 2 users, each one should get 50% of rewards
    assert_eq!(
        to_int(view!(
            lockup_stake.get_account_total_balance(user1.account_id())
        )),
        to_yocto("10003")
    );
}

/// Tests lockup_stake, depositing from regular account and from lockup-account.
#[test]
fn test_stake_with_lockup() {
    let (root, lockup_stake, lockup) = setup();

    let user1 = create_user_and_stake("user1".into(), &root, &lockup_stake);
    let user1_stake = 10000 * NEAR;

    assert_between(
        to_int(view!(
            lockup_stake.get_account_total_balance(user1.account_id())
        )),
        "9999.99",
        "10000.01",
    );

    assert_between(
        to_int(view!(
            lockup_stake.get_account_total_balance(root.account_id())
        )),
        "0",
        "0.01",
    );

    wait_epoch(&root);
    assert_all_success(call!(root, lockup_stake.ping()));

    let lockup_account_balance_pre = lockup.account().unwrap().amount;
    println!("lockup account_balance {}", lockup_account_balance_pre);

    storage_register(&root, lockup_account_id());
    call(
        &root,
        lockup_account_id(),
        "select_staking_pool",
        json!({ "staking_pool_account_id": LOCKUP_STAKE_CONTRACT_ID }),
        0,
        0,
    );
    let lockup_acc_stake_yoctos = 50000 * NEAR;
    call(
        &root,
        lockup_account_id(),
        "deposit_and_stake",
        json!({ "amount": lockup_acc_stake_yoctos.to_string() }),
        0,
        125 * TGAS,
    );
    println!(
        "{:?}",
        root.borrow_runtime().view_account(LOCKUP_STAKE_CONTRACT_ID)
    );
    assert_all_success(call!(root, lockup_stake.ping()));

    let lockup_account_balance_mid = lockup.account().unwrap().amount;
    println!("lockup account_balance_mid {}", lockup_account_balance_mid);
    assert_eq!(
        lockup_account_balance_mid,
        lockup_account_balance_pre - lockup_acc_stake_yoctos
    );

    simulate_st_near_rewards(&root, 6);
    // before ping
    assert_eq!(
        to_int(view!(
            lockup_stake.get_account_total_balance(user1.account_id())
        )),
        user1_stake
    );
    // 2 users, one gets 1 the other 5
    println!(
        "before ping {} {}",
        to_int(view!(lockup_stake.get_account_total_balance(lockup_account_id()))),
        to_int(view!(lockup_stake.get_account_total_balance(lockup_account_id())))
    );
    // PING
    assert_all_success(call!(root, lockup_stake.ping()));
    // after ping
    println!(
        "after ping {} {}",
        to_int(view!(lockup_stake.get_account_total_balance(lockup_account_id()))),
        to_int(view!(lockup_stake.get_account_total_balance(lockup_account_id())))
    );
    let user1_stake_plus_rewards = user1_stake + 1 * NEAR;
    assert_eq!(
        to_int(view!(
            lockup_stake.get_account_total_balance(user1.account_id())
        )),
        user1_stake_plus_rewards
    );

    assert_eq!(
        to_int(view!(lockup_stake.get_account_shares(lockup_account_id()))),
        lockup_acc_stake_yoctos
    );
    // 2 users, one gets 1 the other 5
    let lockup_acc_stake_plus_rewards = lockup_acc_stake_yoctos + 5 * NEAR;
    assert_eq!(
        to_int(view!(lockup_stake.get_account_total_balance(lockup_account_id()))),
        lockup_acc_stake_plus_rewards
    );

    // UNSTAKE
    call(
        &root,
        lockup_account_id(),
        "unstake",
        json!({ "amount": lockup_acc_stake_plus_rewards.to_string() }),
        0,
        125 * TGAS,
    );

    assert_eq!(
        to_int(view!(lockup_stake.get_account_shares(lockup_account_id()))),
        to_yocto("0")
    );
    assert_eq!(
        to_int(view!(lockup_stake.get_account_total_balance(lockup_account_id()))),
        lockup_acc_stake_plus_rewards
    );

    // withdraw_all_from_staking_pool  -- should fail
    call_some_fail(
        &root,
        lockup_account_id(),
        "withdraw_all_from_staking_pool",
        json!({}),
        0,
    );

    wait_epoch(&root);
    wait_epoch(&root);
    wait_epoch(&root);
    wait_epoch(&root);

    // simulate meta-lockup_stake retrieval of funds
    // you must compile the "with-tests-functions" branch of the liquid staking contract to enable this simulation functions
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
        lockup_account_id(),
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
fn test_stake_with_lockup_fail_paths() {
    let (root, lockup_stake, lockup) = setup();

    let user1 = create_user_and_stake("user1".into(), &root, &lockup_stake);
    let user1_stake = 10000 * NEAR;

    assert_between(
        to_int(view!(
            lockup_stake.get_account_total_balance(user1.account_id())
        )),
        "9999.99",
        "10000.01",
    );

    assert_between(
        to_int(view!(
            lockup_stake.get_account_total_balance(root.account_id())
        )),
        "0",
        "0.01",
    );

    wait_epoch(&root);
    // PING
    assert_all_success(call!(root, lockup_stake.ping()));

    let lockup_account_balance_pre = lockup.account().unwrap().amount;
    println!("lockup account_balance {}", lockup_account_balance_pre);

    call(
        &root,
        lockup_account_id(),
        "select_staking_pool",
        json!({ "staking_pool_account_id": LOCKUP_STAKE_CONTRACT_ID }),
        0,
        0,
    );
    let lockup_acc_stake_yoctos = 50000 * NEAR;
    storage_register(&root, lockup_account_id());
    call(
        &root,
        lockup_account_id(),
        "deposit_and_stake",
        json!({ "amount": lockup_acc_stake_yoctos.to_string() }),
        0,
        125 * TGAS,
    );
    println!(
        "{:?}",
        root.borrow_runtime().view_account(LOCKUP_STAKE_CONTRACT_ID)
    );
    assert_all_success(call!(root, lockup_stake.ping()));

    let lockup_account_balance_mid = lockup.account().unwrap().amount;
    println!("lockup account_balance_mid {}", lockup_account_balance_mid);
    assert_eq!(
        lockup_account_balance_mid,
        lockup_account_balance_pre - lockup_acc_stake_yoctos
    );

    simulate_st_near_rewards(&root, 6);
    // before ping
    assert_eq!(
        to_int(view!(
            lockup_stake.get_account_total_balance(user1.account_id())
        )),
        user1_stake
    );
    // 2 users, one gets 1 the other 5
    // PING
    assert_all_success(call!(root, lockup_stake.ping()));
    // after ping
    let user1_stake_plus_rewards = user1_stake + 1 * NEAR;
    assert_eq!(
        to_int(view!(
            lockup_stake.get_account_total_balance(user1.account_id())
        )),
        user1_stake_plus_rewards
    );

    assert_eq!(
        to_int(view!(lockup_stake.get_account_shares(lockup_account_id()))),
        lockup_acc_stake_yoctos
    );
    // 2 users, one gets 1 the other 5
    let lockup_acc_stake_plus_rewards = lockup_acc_stake_yoctos + 5 * NEAR;
    assert_eq!(
        to_int(view!(lockup_stake.get_account_total_balance(lockup_account_id()))),
        lockup_acc_stake_plus_rewards
    );

    // // simulate the inner promise failure
    // st_near_set_busy(&root, true);

    // UNSTAKE, inner promise should fail
    call_some_fail(
        &root,
        lockup_account_id(),
        "unstake",
        json!({ "amount": (lockup_acc_stake_plus_rewards + 1_000_000 * NEAR).to_string() }),
        0,
    );

    // // simulate the inner promise failure
    // st_near_set_busy(&root, false);

    // UNSTAKE
    call(
        &root,
        lockup_account_id(),
        "unstake",
        json!({ "amount": lockup_acc_stake_plus_rewards.to_string() }),
        0,
        125 * TGAS,
    );

    assert_eq!(
        to_int(view!(lockup_stake.get_account_shares(lockup_account_id()))),
        to_yocto("0")
    );
    assert_eq!(
        to_int(view!(lockup_stake.get_account_total_balance(lockup_account_id()))),
        lockup_acc_stake_plus_rewards
    );

    // withdraw_all_from_staking_pool  -- should fail
    call_some_fail(
        &root,
        lockup_account_id(),
        "withdraw_all_from_staking_pool",
        json!({}),
        0,
    );

    wait_epoch(&root);
    wait_epoch(&root);
    wait_epoch(&root);
    wait_epoch(&root);

    // simulate meta-lockup_stake retrieval of funds
    // you must compile the "for-tests" branch of meta lockup_stake liquid staking contract to enable this simulation functions

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
        lockup_account_id(),
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
    let (root, lockup_stake, lockup) = setup();

    let user1 = create_user_and_stake("user1".into(), &root, &lockup_stake);
    let user1_stake = 10000 * NEAR;

    let initial_user2_deposit = 10100 * NEAR;
    let user2 = create_user_and_metapool("user2".into(), initial_user2_deposit, &root);
    let user2_balance_metapool = balance_nears_metapool(&user2);
    assert_eq!(user2_balance_metapool, initial_user2_deposit);

    let user2_balance_shares = balance_shares_metapool(&user2);
    assert_eq!(user2_balance_shares, initial_user2_deposit);

    wait_epoch(&root);
    // PING
    assert_all_success(call!(root, lockup_stake.ping()));

    // add liquidity
    call(
        &root,
        meta_pool_contract_id(),
        "nslp_add_liquidity",
        json!({}),
        500 * NEAR,
        50 * TGAS,
    );
    // liquid unstake, adds stNEAR to the lockup_stake
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
        lockup_account_id(),
        "select_staking_pool",
        json!({ "staking_pool_account_id": LOCKUP_STAKE_CONTRACT_ID }),
        0,
        0,
    );
    storage_register(&root, lockup_account_id());
    let lockup_acc_stake_yoctos = 50000 * NEAR;
    call(
        &root,
        lockup_account_id(),
        "deposit_and_stake",
        json!({ "amount": lockup_acc_stake_yoctos.to_string() }),
        0,
        125 * TGAS,
    );
    println!(
        "{:?}",
        root.borrow_runtime().view_account(LOCKUP_STAKE_CONTRACT_ID)
    );
    assert_all_success(call!(root, lockup_stake.ping()));

    let lockup_account_balance_mid = lockup.account().unwrap().amount;
    println!("lockup account_balance_mid {}", lockup_account_balance_mid);
    assert_eq!(
        lockup_account_balance_mid,
        lockup_account_balance_pre - lockup_acc_stake_yoctos
    );

    simulate_st_near_rewards(&root, 7);
    // before ping
    assert_eq!(
        to_int(view!(
            lockup_stake.get_account_total_balance(user1.account_id())
        )),
        user1_stake
    );
    // 3 users
    // PING
    assert_all_success(call!(root, lockup_stake.ping()));
    // after ping
    let user1_stake_plus_rewards = user1_stake + 1 * NEAR;
    assert_tolerance(
        to_int(view!(
            lockup_stake.get_account_total_balance(user1.account_id())
        )),
        user1_stake_plus_rewards,
        10000,
    );

    assert_tolerance(
        to_int(view!(lockup_stake.get_account_shares(lockup_account_id()))),
        lockup_acc_stake_yoctos,
        10000,
    );
    // 2 users, one gets 1 the other 5
    let lockup_acc_stake_plus_rewards = lockup_acc_stake_yoctos + 5 * NEAR;
    assert_tolerance(
        to_int(view!(lockup_stake.get_account_total_balance(lockup_account_id()))),
        lockup_acc_stake_plus_rewards,
        10000,
    );

    // simulate the inner promise failure
    st_near_set_busy(&root, true);

    // UNSTAKE, inner promise should fail
    call_some_fail(
        &root,
        lockup_account_id(),
        "unstake",
        json!({ "amount": lockup_acc_stake_plus_rewards.to_string() }),
        0,
    );

    // simulate the inner promise failure
    st_near_set_busy(&root, false);

    // UNSTAKE
    call(&root, lockup_account_id(), "unstake_all", json!({}), 0, 125 * TGAS);

    assert_eq!(
        to_int(view!(lockup_stake.get_account_shares(lockup_account_id()))),
        to_yocto("0")
    );
    assert_tolerance(
        to_int(view!(lockup_stake.get_account_total_balance(lockup_account_id()))),
        lockup_acc_stake_plus_rewards,
        10000,
    );

    wait_epoch(&root);
    wait_epoch(&root);
    wait_epoch(&root);
    wait_epoch(&root);

    // simulate meta-lockup_stake retrieval of funds
    // you must compile the "for-tests" branch of meta lockup_stake liquid staking contract to enable this simulation functions

    call(
        &root,
        meta_pool_contract_id(),
        "test_simulate_retrieval",
        json!({}),
        lockup_acc_stake_plus_rewards,
        0,
    );

    // withdraw_all_from_staking_pool  -- should succeed
    call_some_fail(
        &root,
        lockup_account_id(),
        "withdraw_from_staking_pool",
        json!({"amount": (1_000_000 * NEAR).to_string()}),
        0,
    );

    // withdraw_all_from_staking_pool  -- should succeed
    // Requires 175 TGas (7 * BASE_GAS) according to lockup contract
    call(
        &root,
        lockup_account_id(),
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
        10000,
    );
}
