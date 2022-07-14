use crate::bit_ops::{get_bit, set_bit};

/// Encodes the high bits of `bytes` into an extra high byte
pub(crate) fn encode_high_byte<const N: usize>(mut bytes: [u8; N]) -> (u8, [u8; N]) {
    let mut high_byte = 0b10000000u8;
    for (index, byte) in bytes.iter_mut().enumerate() {
        set_bit(&mut high_byte, index, get_bit(*byte, 7));
        set_bit(byte, 7, true);
    }
    (high_byte, bytes)
}

/// Applies the high bits to `bytes`
///
/// Also verifies that all original bytes have their high bit set. On failure returns the index of
/// the first invalid byte.
pub(crate) fn decode_high_byte<const N: usize>(
    (high_byte, mut bytes): (u8, [u8; N]),
) -> Result<[u8; N], usize> {
    fn check_bit(byte: u8, expected: bool, index: usize) -> Result<(), usize> {
        if get_bit(byte, 7) != expected {
            Err(index)
        } else {
            Ok(())
        }
    }
    check_bit(high_byte, true, 0)?;
    for (index, byte) in bytes.iter_mut().enumerate() {
        check_bit(*byte, true, index + 1)?;
        set_bit(byte, 7, get_bit(high_byte, index));
    }
    Ok(bytes)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_encode_high_byte() {
        assert_eq!(
            encode_high_byte([0x00, 0xFF, 0x00, 0xFF]),
            (0b10001010, [0x80, 0xFF, 0x80, 0xFF])
        )
    }

    #[test]
    fn test_decode_high_byte() {
        assert_eq!(decode_high_byte((0b10001010, [0x80, 0xFF, 0x80, 0xFF])).unwrap(), [
            0x00, 0xFF, 0x00, 0xFF
        ])
    }

    #[test]
    fn test_high_byte() {
        let raw = (0b10001010, [0x80, 0xFF, 0x80, 0xFF]);
        assert_eq!(encode_high_byte(decode_high_byte(raw).unwrap()), raw);
        let decoded = [0x00, 0xFF, 0x00, 0xFF];
        assert_eq!(decode_high_byte(encode_high_byte(decoded)).unwrap(), decoded);
    }
}
