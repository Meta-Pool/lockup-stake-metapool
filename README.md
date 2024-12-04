# Stake Lockup Accounts with Meta Pool

The `Lockup-Stake-MetaPool` contract allows lockup account owners to stake in Meta Pool obtaining the risk reduction benefit
of automatically distributing the stake into the most performant and stable validators.

Lockup-Stake-MetaPool works as a normal stake pool contract, accepting Lockup-accounts while keeping the added benefit of automatically keeping the stake delegated into performant validators.

The contract is non-custodial, and the owner can unstake the funds with the standard four epochs delay.

## Audit
* [Blocksec Audit Report](docs/blocksec_lockup_v1.0-signed-audit.pdf)

## Mainnet contract address
* `lockup-meta-pool.near`

## Instructions
### To Build the wasm

1. run `bash build.sh`

### Local Integration Test

1. Compile Meta Pool [liquid staking contract](https://github.com/Meta-Pool/liquid-staking-contract), using the branch named: `with-test-functions`
2. copy the generated `metapool.wasm` into the `/res` folder
`cp target/wasm32-unknown-unknown/release/metapool.wasm ../lockup-stake-metapool/res/`
3. run `cargo test`

### Testnet contract address

* `lockup.meta-v2.pool.testnet`
