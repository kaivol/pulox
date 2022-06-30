//! Packages sent by the device.

use core::fmt;
use core::fmt::{Debug, Formatter};
use core::task::Poll;

use futures::ready;

use crate::bit_ops::{get_bit, get_bit_range};
use crate::encoding::decode_high_byte;
use crate::Error;

macro_rules! incoming_packages {
    (
        $(
            $(#[$outer:meta])*
            $code:literal => |$bytes:ident: [u8; $length:literal]| $(#[$outer2:meta])* $name:ident {
                $(
                    $(#[$field_meta:meta])*
                    $field_vis:vis $field_name:ident: $field_type:ty = $field_const:expr
                ),*$(,)?
            }
        ),*$(,)?
    ) => {
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
            $(#[$outer2])*
            pub struct $name {
                $(
                    $(#[$field_meta])*
                    $field_vis $field_name: $field_type,
                )*
            }

            impl $name {
                /// Create a new Package from the given byte array
                pub(super) fn from_bytes($bytes: [u8; $length]) -> Self {
                    $name {
                        $($field_name: $field_const,)*
                    }
                }
            }
        )*

        /// State machine which handles incoming packages
        pub enum IncomingStateMachine {
            /// Current status: nothing read
            None,
            $(
                $(#[$outer])*
                $name {
                    /// byte buffer
                    buffer: [u8; ($length+1)],
                    /// byte count
                    received_bytes: usize
                },
            )*
        }

        impl IncomingStateMachine {
            /// Resumes execution of the state machine
            pub fn resume<
                #[cfg(feature = "std")] E: snafu::AsErrorSource,
                #[cfg(not(feature = "std"))] E,
            >(&mut self, mut read: impl FnMut(&mut [u8]) -> Poll<core::result::Result<usize, E>>) -> Poll<$crate::Result<IncomingPackage, E>> {
                loop {
                    match self {
                        IncomingStateMachine::None => {
                            let mut code = [0u8];
                            let count = ready!(read(&mut code))?;
                            if count == 0 {
                                return Err(Error::DeviceReadZero).into();
                            }
                            if count > 1 {
                                return Err(Error::DeviceReadTooMuch { requested: 1, reported: count }).into();
                            }
                            match code[0] {
                                $(
                                    $code => *self = IncomingStateMachine::$name {
                                        buffer: [0; ($length + 1)],
                                        received_bytes: 0
                                    },
                                )*
                                code => return Err(Error::UnknownTypeCode{ code }).into(),
                            }
                        },
                        $(
                            IncomingStateMachine::$name { ref mut buffer, ref mut received_bytes } => {
                                let slice = &mut buffer[*received_bytes..($length + 1)];
                                let count = ready!(read(slice))?;
                                if count == 0 {
                                    return Err(Error::DeviceReadZero).into();
                                }
                                if count > slice.len() {
                                    return Err(Error::DeviceReadTooMuch {
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

                                    *self = IncomingStateMachine::None;

                                    return Poll::Ready(Ok(IncomingPackage::$name(data)))
                                }
                            },
                        )*
                    }
                }
            }
        }
    };
}

incoming_packages! {
    /// Real time data
    0x01 => |bytes: [u8; 7]| #[derive(Debug, Copy, Clone)] RealTimeData {
        /// Signal strength
        pub signal_strength: u8 = get_bit_range(bytes[0], 0..=3),
        /// Searching time too long
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
    0x04 => |bytes: [u8; 7]| #[derive(Debug, Copy, Clone)] DeviceIdentifier {
        /// Identifier
        pub identifier: [u8; 7] = bytes,
    },
    /// User Information
    0x05 => |bytes: [u8; 7]| #[derive(Debug, Copy, Clone)] UserInformation {
        /// User Index Number
        pub user_index: u8 = bytes[0],
        /// User Information
        pub user_info: [u8; 6] = [bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6]]
    },
    /// Storage start time(date)
    0x07 => |bytes: [u8; 6]| #[derive(Debug, Copy, Clone)] StorageStartTimeDate {
        /// User Index Number
        pub user_index: u8 = bytes[0],
        /// Storage Segment Number
        pub storage_segment: u8 = bytes[1],
        /// Year
        pub year: u16 = (bytes[2] as u16) + ((bytes[3] as u16) << 8),
        /// Month
        pub month: u8 = bytes[4],
        /// Day
        pub day: u8 = bytes[5],
    },
    /// Storage start time(time)
    0x12 => |bytes: [u8; 6]| #[derive(Debug, Copy, Clone)] StorageStartTimeTime {
        /// User Index Number
        pub user_index: u8 = bytes[0],
        /// Storage Segment Number
        pub storage_segment: u8 = bytes[1],
        /// Hour
        pub hour: u8 = bytes[2],
        /// Minutes
        pub minute: u8 = bytes[3],
        /// Seconds
        pub second: u8 = bytes[4],
    },
    /// Storage Data Length
    0x08 => |bytes: [u8; 6]| #[derive(Debug, Copy, Clone)] StorageDataLength {
        /// User Index Number
        pub user_index: u8 = bytes[0],
        /// Data Segment Number
        pub data_segment: u8 = bytes[1],
        /// Data segment length
        pub length: u32 =
            (bytes[2] as u32) + ((bytes[3] as u32) << 8) + ((bytes[4] as u32) << 16) + ((bytes[5] as u32) << 24),
    },
    /// Storage Data with PI
    0x09 => |bytes: [u8; 4]| #[derive(Debug, Copy, Clone)] StorageDataWithPI {
        /// SpO2
        pub spo2: u8 = bytes[0],
        /// Pulse rate
        pub pulse_rate: u8 = bytes[1],
        /// Perfusion Index
        pub pi: u16 = (bytes[2] as u16) + ((bytes[3] as u16) << 8),
    },
    /// Storage Data Segment Amount
    0x0A => |bytes: [u8; 2]| #[derive(Debug, Copy, Clone)] StorageDataSegmentAmount {
        /// User Index Number
        pub user_index: u8 = bytes[0],
        /// Segment Amount
        pub segment_amount: u8 = bytes[1],
    },
    /// Command Feedback
    0x0B => |bytes: [u8; 2]| CommandFeedback {
        /// Command
        pub command: u8 = bytes[0],
        /// Reason Code
        pub code: u8 = bytes[1],
    },
    /// Device free feedback
    0x0C => |_bytes: [u8; 0]| #[derive(Debug, Copy, Clone)] FreeFeedback {},
    /// Device disconnect notice
    0x0D => |bytes: [u8; 1]| #[derive(Debug, Copy, Clone)] DisconnectNotice {
        /// Disconnect reason
        pub reason: u8 = bytes[0],
    },
    /// PI Identifiers
    0x0E => |bytes: [u8; 1]| #[derive(Debug, Copy, Clone)] PIIdentifiers {
        /// Whether to support PI in real-time data
        pub pi_support: u8 = bytes[0],
    },
    /// Storage Data
    0x0F => |bytes: [u8; 6]| #[derive(Debug, Copy, Clone)] StorageData {
        /// SpO2 entry 1
        pub spo2_1: u8 = bytes[0],
        /// Pulse rate entry 1
        pub pulse_rate_1: u8 = bytes[1],
        /// SpO2 entry 2
        pub spo2_2: u8 = bytes[2],
        /// Pulse rate entry 2
        pub pulse_rate_2: u8 = bytes[3],
        /// SpO2 entry 3
        pub spo2_3: u8 = bytes[4],
        /// Pulse rate entry 3
        pub pulse_rate_3: u8 = bytes[5],
    },
    /// User Amount
    0x10 => |bytes: [u8; 1]| #[derive(Debug, Copy, Clone)] UserAmount {
        /// Total User Number
        pub total_user: u8 = bytes[0],
    },
    /// Device Notice
    0x11 => |bytes: [u8; 7]| #[derive(Debug, Copy, Clone)] DeviceNotice {
        /// Device Notice Type
        pub device_notice: u8 = bytes[0],
        /// Notice Information
        pub device_info: [u8; 6] = [bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6]],
    },
    /// Storage Data Identifiers
    0x15 => |bytes: [u8; 7]| #[derive(Debug, Copy, Clone)] StorageDataIdentifiers {
        /// User Index Number
        pub user_index: u8 = bytes[0],
        /// Data Segment Number
        pub data_segment: u8 = bytes[1],
        /// PI Identifiers
        pub pi_identifiers: u8 = bytes[2],
        /// Retention
        pub retention: [u8; 4] = [bytes[3], bytes[4], bytes[5], bytes[6]],
    },
}

impl CommandFeedback {
    /// Meaning of this device command feedback
    pub fn message(&self) -> &str {
        match self.code {
            0x00 => "Completed operation",
            0x01 => "Shutdown device",
            0x02 => "Exchange users",
            0x03 => "Recording",
            0x04 => "Failure to delete the storage data",
            0x05 => "Not supported",
            _ => "Unknown reason",
        }
    }
}

impl Debug for CommandFeedback {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("CommandFeedback")
            .field("command", &format_args!("{:#04X}", self.command))
            .field("reason_code", &format_args!("{:#04X}", self.code))
            .field("message", &format_args!("'{}'", self.message()))
            .finish()
    }
}
