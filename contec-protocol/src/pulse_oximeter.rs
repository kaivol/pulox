use crate::bit_ops::{get_bit, get_bit_range, set_bit};
use crate::incoming_package::RealTimeData;
use crate::{Error, Result};
use core::pin::Pin;
use core::task::Poll;
use futures::ready;
use futures::{future, AsyncRead, AsyncWrite, Future};

/// Represents a connection with a pulse oximeter.
///
/// Use the [PulseOximeter::send_package()] and [PulseOximeter::receive_package] methods to
/// communicate with the foobar device.
pub struct PulseOximeter<T>
where
    T: AsyncRead,
    T: AsyncWrite,
    T: Unpin,
{
    port: T,
    incoming: IncomingStatus,
    outgoing: OutgoingStatus,
}

enum OutgoingStatus {
    None,
    Some {
        buffer: [u8; 9],
        already_sent: usize,
    },
}

pub(crate) mod private {
    pub trait OutgoingPackage {
        const CODE: u8;
        fn bytes(&self) -> [u8; 7];
    }
}

impl<T: AsyncRead + AsyncWrite + Unpin> PulseOximeter<T> {
    /// Create a pulse oximeter interface, using the given `port` for communication.
    pub fn new(port: T) -> Self {
        Self {
            port,
            incoming: IncomingStatus::None,
            outgoing: OutgoingStatus::None,
        }
    }

    /// Send a package to the device.
    ///
    /// Note that if a future returned by a previous call to this function was not polled until
    /// completion, the rest of the package of the previous call will be sent before the new
    /// package will be sent.
    #[allow(clippy::bool_assert_comparison)]
    pub fn send_package<P>(&mut self, package: P) -> impl Future<Output = Result<()>> + '_
    where
        P: private::OutgoingPackage,
    {
        debug_assert_eq!(get_bit(P::CODE, 7), false);

        let (high_byte, data) = encode_high_byte(package.bytes());

        let mut buffer = [0; 9];
        buffer[0] = P::CODE;
        buffer[1] = high_byte;
        buffer[2..9].copy_from_slice(&data);

        let unfinished_send = matches!(self.outgoing, OutgoingStatus::Some { .. });

        future::poll_fn(move |cx| loop {
            match self.outgoing {
                OutgoingStatus::None => {
                    self.outgoing = OutgoingStatus::Some {
                        buffer,
                        already_sent: 0,
                    }
                }
                OutgoingStatus::Some {
                    buffer,
                    ref mut already_sent,
                } => {
                    let slice = &buffer[*already_sent..9];
                    let bytes_written = ready!(Pin::new(&mut self.port).poll_write(cx, slice))?;
                    if bytes_written == 0 {
                        return Err(Error::DeviceWriteZero).into();
                    }
                    if bytes_written > slice.len() {
                        return Err(Error::DeviceWriteTooMuch {
                            requested: slice.len(),
                            reported: bytes_written,
                        })
                        .into();
                    }
                    *already_sent += bytes_written;
                    if *already_sent == 9 {
                        self.outgoing = OutgoingStatus::None;
                        if !unfinished_send {
                            return Poll::Ready(Ok(()));
                        }
                    }
                }
            }
        })
    }
}

/// Encodes the high bits of `bytes` into an extra high byte
fn encode_high_byte<const N: usize>(mut bytes: [u8; N]) -> (u8, [u8; N]) {
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
fn decode_high_byte<const N: usize>(
    (high_byte, mut bytes): (u8, [u8; N]),
) -> core::result::Result<[u8; N], usize> {
    fn check_bit(byte: u8, expected: bool, index: usize) -> core::result::Result<(), usize> {
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

macro_rules! incoming_packages {
    (
        $(
            $(#[$outer:meta])*
            $code:literal => |$bytes:ident: [u8; $length:literal]| $name:ident {
                $(
                    $(#[$field_meta:meta])*
                    $field_vis:vis $field_name:ident: $field_type:ty = $field_const:expr
                ),*$(,)?
            }
        ),*$(,)?
    ) => {
        /// Packages sent by the device.
        pub mod incoming_package {
            use super::*;

            /// A Package sent by the device.
            #[derive(Debug)]
            pub enum IncomingPackage {
                $(
                    $(#[$outer])*
                    $name($name),
                )*
            }

            $(
                $(#[$outer])*
                #[derive(Debug)]
                pub struct $name {
                    $(
                        $(#[$field_meta])*
                        $field_vis $field_name: $field_type,
                    )*
                }

                impl $name {
                    /// Create a new Package from the given byte array
                    pub fn from_bytes($bytes: [u8; $length]) -> Self {
                        $name {
                            $($field_name: $field_const,)*
                        }
                    }
                }
            )*

            pub(super) enum IncomingStatus {
                None,
                $(
                    $name { buffer: [u8; ($length+1)], received_bytes: usize },
                )*
            }
        }
        use incoming_package::*;

        impl<T : AsyncRead + AsyncWrite + Unpin> PulseOximeter<T> {
            /// Receive the next package from the device.
            pub fn receive_package(&mut self) -> impl Future<Output = Result<IncomingPackage>> + '_ {
                future::poll_fn(|cx| loop {
                    match self.incoming {
                        IncomingStatus::None => {
                            let mut code = [0u8];
                            let count = ready!(Pin::new(&mut self.port).poll_read(cx, &mut code))?;
                            if count == 0 {
                                return Err(Error::DeviceReadZero).into();
                            }
                            if count > 1 {
                                return Err(Error::DeviceReadTooMuch { requested: 1, reported: count }).into();
                            }
                            match code[0] {
                                $(
                                    $code => self.incoming = IncomingStatus::$name {
                                        buffer: [0; ($length + 1)],
                                        received_bytes: 0
                                    },
                                )*
                                c => return Err(Error::UnknownTypeCode(c)).into(),
                            }
                        },
                        $(
                            IncomingStatus::$name { ref mut buffer, ref mut received_bytes } => {
                                let slice = &mut buffer[*received_bytes..($length + 1)];
                                let count =  ready!(Pin::new(&mut self.port).poll_read(cx, slice))?;
                                if count == 0 {
                                    return Err(Error::DeviceReadZero).into();
                                }
                                if count > slice.len() {
                                    return  Err(Error::DeviceReadTooMuch {
                                        requested: slice.len(),
                                        reported: count
                                    }).into();
                                }
                                *received_bytes += count;
                                if *received_bytes == ($length + 1) {
                                    let [high_byte, data @ ..] = *buffer;
                                    let decoded = match decode_high_byte((high_byte, data)){
                                        Ok(decoded) => decoded,
                                        Err(invalid_index) => {
                                            let mut bytes = [0; 8];
                                            bytes[..$length+1].copy_from_slice(buffer);
                                            return Err(Error::InvalidPackageData {
                                                code: $code,
                                                bytes,
                                                length: $length+1,
                                                invalid_index
                                            }).into();
                                        }
                                    };
                                    let data = $name::from_bytes(decoded);

                                    self.incoming = IncomingStatus::None;

                                    return Poll::Ready(Ok(IncomingPackage::$name(data)))
                                }
                            },
                        )*
                    }
                })
            }
        }
    };
}

incoming_packages! {
    /// Real time data
    0x01 => |bytes: [u8; 7]| RealTimeData {
        /// Signal strength
        pub signal_strength: u8 = get_bit_range(bytes[0], 0..=3),
        /// Searhcing time too long
        pub searching_time_too_long: bool = get_bit(bytes[0], 4),
        /// Low SpO2
        pub low_spo2: bool = get_bit(bytes[0], 5),
        /// Pulse beep
        pub pulse_beep: bool = get_bit(bytes[0], 6),
        /// Probe errors
        pub probe_errors: bool = get_bit(bytes[0], 7),
        /// Pulse waveform
        pub pulse_waveform: u8 = get_bit_range(bytes[1], 0..=6),
        /// Searching pulse
        pub searching_pulse: bool = get_bit(bytes[1], 7),
        /// Bar graph
        pub bar_graph: u8 = get_bit_range(bytes[2], 0..=3),
        /// PI invalid
        pub pi_invalid: bool = get_bit(bytes[2], 4),
        /// Pulse rate
        pub pulse_rate: u8 = bytes[3],
        /// SpO2
        pub spo2: u8 = bytes[4],
        /// PI
        pub pi: u16 = (bytes[5] as u16) + ((bytes[6] as u16) << 8)
    },
    /// Device identifier
    0x04 => |bytes: [u8; 7]| DeviceIdentifier {
        /// Identifier
        pub identifier: [u8; 7] = bytes,
    },
    /// Device free feedback
    0x0C => |_bytes: [u8; 0]| FreeFeedback {},
    /// Device disconnect notice
    0x0D => |bytes: [u8; 1]| DisconnectNotice {
        /// Disconnect reason
        pub reason: u8 = bytes[0],
    },
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
        assert_eq!(
            decode_high_byte((0b10001010, [0x80, 0xFF, 0x80, 0xFF])).unwrap(),
            [0x00, 0xFF, 0x00, 0xFF]
        )
    }

    #[test]
    fn test_high_byte() {
        let raw = (0b10001010, [0x80, 0xFF, 0x80, 0xFF]);
        assert_eq!(encode_high_byte(decode_high_byte(raw).unwrap()), raw);
        let decoded = [0x00, 0xFF, 0x00, 0xFF];
        assert_eq!(
            decode_high_byte(encode_high_byte(decoded)).unwrap(),
            decoded
        );
    }
}
