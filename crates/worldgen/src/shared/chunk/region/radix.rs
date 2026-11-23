const DIGIT_BITS: u32 = 11;
const BUCKETS: usize = 1 << DIGIT_BITS;
const KEY_BITS: u32 = 2 * DIGIT_BITS;
const SMALL: usize = 256;

pub(super) fn sort_stable_by_key<T: Copy>(items: &mut [T], buffer: &mut Vec<T>, key: impl Fn(&T) -> u32) {
    if items.len() < SMALL || items.iter().any(|item| key(item) >> KEY_BITS != 0) {
        items.sort_by_key(|item| key(item));
        return;
    }
    let mask = BUCKETS as u32 - 1;
    let mut low = [0u32; BUCKETS];
    let mut high = [0u32; BUCKETS];
    for item in items.iter() {
        let key = key(item);
        low[(key & mask) as usize] += 1;
        high[(key >> DIGIT_BITS) as usize] += 1;
    }
    for counts in [&mut low, &mut high] {
        let mut total = 0;
        for count in counts.iter_mut() {
            let here = *count;
            *count = total;
            total += here;
        }
    }
    buffer.clear();
    buffer.extend_from_slice(items);
    for item in items.iter() {
        let slot = &mut low[(key(item) & mask) as usize];
        buffer[*slot as usize] = *item;
        *slot += 1;
    }
    for item in buffer.iter() {
        let slot = &mut high[(key(item) >> DIGIT_BITS) as usize];
        items[*slot as usize] = *item;
        *slot += 1;
    }
}
