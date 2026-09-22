pub const fn compute_required_bits(min: i32, max: i32) -> u8 {
    let val = max - min;
    if val <= 1 {
        return 1;
    }

    let mut bits: u8 = 1;
    while val >= (1 << bits) {
        bits += 1;
    }
    bits
}
