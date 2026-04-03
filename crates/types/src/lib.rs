pub mod constants;
pub mod error;
pub mod gr;
pub mod disjointset;

#[inline(always)]
pub fn yes_or_no(input: bool) -> String {
    if input {
        String::from(constants::el::YES)
    } else {
        String::from(constants::el::NO)
    }
}

#[inline(always)]
pub fn round(n: f64) -> f64 {
    (n * 10000.0).round() / 10000.0
}
