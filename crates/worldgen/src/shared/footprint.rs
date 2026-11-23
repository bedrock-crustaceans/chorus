pub const fn cells(reach: i32) -> usize {
    let side = (reach * 2 + 1) as usize;
    side * side
}

pub const fn square<const N: usize>(reach: i32) -> [(i32, i32); N] {
    let mut offsets = [(0, 0); N];
    let mut index = 0;
    let mut x = -reach;
    while x <= reach {
        let mut z = -reach;
        while z <= reach {
            offsets[index] = (x, z);
            index += 1;
            z += 1;
        }
        x += 1;
    }
    offsets
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::CHUNK_NEIGHBORHOOD;

    #[test]
    fn a_square_covers_every_offset_within_the_reach_in_row_order() {
        assert_eq!(square::<{ cells(1) }>(1).as_slice(), CHUNK_NEIGHBORHOOD);
        let wide = square::<{ cells(2) }>(2);
        assert_eq!(wide.len(), 25);
        assert_eq!(wide[0], (-2, -2));
        assert_eq!(wide[12], (0, 0));
        assert_eq!(wide[24], (2, 2));
        assert!(wide.iter().all(|&(x, z)| x.abs() <= 2 && z.abs() <= 2));
    }
}
