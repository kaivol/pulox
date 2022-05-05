use crate::bit_ops::{get_bit, get_bit_range, set_bit};
use crate::incoming_package::RealTimeData;
use core::pin::Pin;
use core::task::Poll;
use futures::io::{Result};
use futures::ready;
use futures::{future, AsyncRead, AsyncWrite, Future};

pub struct Pulox<T>
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

pub trait OutgoingPackage {
    const CODE: u8;
    fn bytes(&self) -> [u8; 7];
}

impl<T: AsyncRead + AsyncWrite + Unpin> Pulox<T> {
    pub fn new(port: T) -> Self {
        Self {
            port,
            incoming: IncomingStatus::None,
            outgoing: OutgoingStatus::None,
        }
    }

    pub fn send<P>(&mut self, package: P) -> impl Future<Output = Result<()>> + '_ where P: OutgoingPackage {
        assert_eq!(get_bit(P::CODE, 7), false);

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
                    let count = ready!(
                        Pin::new(&mut self.port).poll_write(cx, &buffer[*already_sent..9])
                    )?;
                    assert!(count > 0);
                    *already_sent += count;
                    assert!(*already_sent <= 9);
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

fn encode_high_byte<const N: usize>(mut bytes: [u8; N]) -> (u8, [u8; N]) {
    let mut high_byte = 0b10000000u8;
    for (index, byte) in bytes.iter_mut().enumerate() {
        set_bit(&mut high_byte, index, get_bit(*byte, 7));
        set_bit(byte, 7, true);
    }
    (high_byte, bytes)
}

fn decode_high_byte<const N: usize>((high_byte, mut bytes): (u8, [u8; N])) -> [u8; N] {
    assert_eq!(get_bit(high_byte, 7), true);
    for (index, byte) in bytes.iter_mut().enumerate() {
        assert_eq!(get_bit(*byte, 7), true);
        set_bit(byte, 7, get_bit(high_byte, index));
    }
    bytes
}

macro_rules! incoming_packages {
    (
        $(
            $code:literal => |$bytes:ident: [u8; $length:literal]| $name:ident {
                $($field_name:ident: $field_type:ty = $field_const:expr),*$(,)?
            }
        ),*$(,)?
    ) => {
        pub mod incoming_package {
            use super::*;

            #[derive(Debug)]
            pub enum IncomingPackage {
                $(
                    $name($name),
                )*
            }

            $(
                #[derive(Debug)]
                pub struct $name {
                    $(pub $field_name: $field_type,)*
                }

                impl $name {
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

        impl<T : AsyncRead + AsyncWrite + Unpin> Pulox<T> {
            pub fn next_package(&mut self) -> impl Future<Output = Result<IncomingPackage>> + '_ {
                future::poll_fn(|cx| {
                    loop {
                        match self.incoming {
                            IncomingStatus::None => {
                                let mut code = [0u8];
                                let count = ready!(Pin::new(&mut self.port).poll_read(cx, &mut code))?;
                                assert_eq!(count, 1);
                                match code {
                                    $(
                                        [$code] => self.incoming = IncomingStatus::$name {
                                            buffer: [0; ($length + 1)],
                                            received_bytes: 0
                                        },
                                    )*
                                    c if get_bit(c[0], 7) => todo!(),
                                    _ => todo!(),
                                }
                            },
                            $(
                                IncomingStatus::$name { ref mut buffer, ref mut received_bytes } => {
                                    let slice = &mut buffer[*received_bytes..($length + 1)];
                                    *received_bytes += ready!(Pin::new(&mut self.port).poll_read(cx, slice))?;
                                    if *received_bytes == ($length + 1) {
                                        let [high_byte, data @ ..] = *buffer;
                                        let decoded = decode_high_byte((high_byte, data));
                                        let data = $name::from_bytes(decoded);

                                        self.incoming = IncomingStatus::None;

                                        return Poll::Ready(Ok(IncomingPackage::$name(data)))
                                    }
                                },
                            )*
                        }
                    }
                })
            }
        }
    };
}

incoming_packages! {
    0x01 => |bytes: [u8; 7]| RealTimeData {
        signal_strength: u8 = get_bit_range(bytes[0], 0..=3),
        searching_time_too_long: bool = get_bit(bytes[0], 4),
        low_spo2: bool = get_bit(bytes[0], 5),
        pulse_beep: bool = get_bit(bytes[0], 6),
        probe_errors: bool = get_bit(bytes[0], 7),
        pulse_waveform: u8 = get_bit_range(bytes[1], 0..=6),
        searching_pulse: bool = get_bit(bytes[1], 7),
        bar_graph: u8 = get_bit_range(bytes[2], 0..=3),
        pi_invalid: bool = get_bit(bytes[2], 4),
        pulse_rate: u8 = bytes[3],
        spo2: u8 = bytes[4],
        pi: u16 = (bytes[5] as u16) + ((bytes[6] as u16) << 8)
    },
    0x04 => |bytes: [u8; 7]| DeviceIdentifier {
        identifier: [u8; 7] = bytes,
    },
    0x0C => |_bytes: [u8; 0]| FreeFeedback {},
    0x0D => |bytes: [u8; 1]| DisconnectNotice {
        reason: u8 = bytes[0],
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
            decode_high_byte((0b10001010, [0x80, 0xFF, 0x80, 0xFF])),
            [0x00, 0xFF, 0x00, 0xFF]
        )
    }

    #[test]
    fn test_high_byte() {
        let raw = (0b10001010, [0x80, 0xFF, 0x80, 0xFF]);
        assert_eq!(encode_high_byte(decode_high_byte(raw)), raw);
        let decoded = [0x00, 0xFF, 0x00, 0xFF];
        assert_eq!(decode_high_byte(encode_high_byte(decoded)), decoded);
    }
}
