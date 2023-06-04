use crate::U256;
pub const TGAS: u64 = 1_000_000_000_000;

/// returns amount * numerator/denominator
pub fn mul_div(amount: u128, numerator: u128, denominator: u128) -> u128 {
    return (U256::from(amount) * U256::from(numerator) / U256::from(denominator)).as_u128();
}
