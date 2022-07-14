use core::ops::RangeInclusive;

/// Set the bit at index `index` of `original` to `set`
pub(crate) fn set_bit(original: &mut u8, index: usize, set: bool) {
    if set {
        *original |= 1 << index
    } else {
        *original &= !(1 << index)
    }
}

/// Get bit at index `index` of `original`
pub(crate) const fn get_bit(original: u8, index: usize) -> bool {
    (original & (1 << index)) != 0
}

/// Get the range of bits defined by `range` as `u8`
pub(crate) const fn get_bit_range(original: u8, range: RangeInclusive<usize>) -> u8 {
    let mut mask = 0;
    let mut i = *range.start();
    while i <= *range.end() {
        mask += 1 << i;
        i += 1;
    }
    (original & mask) >> *range.start()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_set_bit() {
        let mut byte = 0b11001100u8;
        set_bit(&mut byte, 0, true);
        assert_eq!(byte, 0b11001101u8);
        set_bit(&mut byte, 7, false);
        assert_eq!(byte, 0b01001101u8);
        set_bit(&mut byte, 3, false);
        set_bit(&mut byte, 4, true);
        assert_eq!(byte, 0b01010101u8);
    }

    #[test]
    fn test_get_bit() {
        assert_eq!(get_bit(0b11001100u8, 0), false);
        assert_eq!(get_bit(0b11001100u8, 3), true);
        assert_eq!(get_bit(0b11001100u8, 7), true);
    }

    #[test]
    fn test_bit_range() {
        assert_eq!(get_bit_range(0b11001100u8, 1..=2), 0b10);
        assert_eq!(get_bit_range(0b11001100u8, 1..=4), 0b0110);
        assert_eq!(get_bit_range(0b11001100u8, 3..=3), 1);
        assert_eq!(get_bit_range(0b11001100u8, 4..=4), 0);
        assert_eq!(get_bit_range(0b11001100u8, 0..=7), 0b11001100u8);
    }
}
