use crate::types::U256;

pub fn mul256(a: u128, b: u128) -> U256 {
    U256::from(a) * U256::from(b)
}

pub fn mul_div_with_rounding(x: U256, y: U256, denominator: U256, rounding_up: bool) -> U256 {
    let mut result = x * y / denominator;
    let remainder = (x * y) % denominator;
    if rounding_up && !remainder.is_zero() {
        result += U256::from(1)
    }
    result
}

pub fn checked_sub(a: u128, b: u128) -> u128 {
    a.checked_sub(b).unwrap_or(0)
}
