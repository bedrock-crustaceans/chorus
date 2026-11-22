use std::cmp::Ordering;

pub fn not_less(a: f32, b: f32) -> bool {
    a.partial_cmp(&b) != Some(Ordering::Less)
}

pub fn not_greater(a: f32, b: f32) -> bool {
    a.partial_cmp(&b) != Some(Ordering::Greater)
}

pub fn not_less_or_equal(a: f32, b: f32) -> bool {
    !matches!(a.partial_cmp(&b), Some(Ordering::Less | Ordering::Equal))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_nan_operand_passes_every_negated_comparison() {
        assert!(not_less(f32::NAN, 1.0) && not_less(1.0, f32::NAN));
        assert!(not_greater(f32::NAN, 1.0) && not_greater(1.0, f32::NAN));
        assert!(not_less_or_equal(f32::NAN, 1.0) && not_less_or_equal(1.0, f32::NAN));
    }
}
