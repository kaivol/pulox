#![allow(clippy::bool_assert_comparison)]

//! Packages sent to the device

use crate::bit_ops::get_bit;
use crate::encoding::encode_high_byte;

/// A package which can be sent to the device
pub trait OutgoingPackage {
    /// The package code
    const CODE: u8;

    /// Gives the 7 data bytes of the package
    fn bytes(&self) -> [u8; 7];
}

/// Gives the byte representation for the package
pub fn bytes_from_package<P>(package: P) -> [u8; 9]
where
    P: OutgoingPackage,
{
    debug_assert_eq!(get_bit(P::CODE, 7), false);

    let (high_byte, data) = encode_high_byte(package.bytes());

    let mut buffer = [0; 9];
    buffer[0] = P::CODE;
    buffer[1] = high_byte;
    buffer[2..9].copy_from_slice(&data);
    buffer
}

/// Control command
pub enum ControlCommand {
    /// Ask device to start sending real time data
    ContinuousRealTimeData,
    /// Stop sending real time data
    StopRealTimeData,
    /// Inform the device that it is still connected
    InformDeviceConnected,
    /// Ask for device identifier
    AskForDeviceIdentifier,
    /// Ask for storage data segment amount
    AskForStorageDataSegmentAmount(u8),
    /// Ask for storage data length
    AskForStorageDataLength(u8, u8),
    /// Ask for storage start time
    AskForStorageStartTime(u8, u8),
    /// Ask for storage data
    AskForStorageData(u8, u8),
    /// Stop sending storage data
    StopStorageData,
    /// Ask for user information
    AskForUserInformation(u8),
    /// Ask for whether to support PI in Real-time data
    AskWhetherSupportPI,
    /// Ask for user amount
    AskForUserAmount,
    /// Delete storage data
    DeleteStorageData(u8, u8),
    /// Ask for storage data identifiers
    AskForStorageDataIdentifiers,
    /// Synchronize device time
    SynchronizeDeviceTime(u8, u8, u8),
    /// Synchronize device date
    SynchronizeDeviceDate(u8, u8, u8, u8, u8),
    /// Ask for storage data identifiers 2
    AskForStorageDataIdentifiers2(u8, u8),
}

impl OutgoingPackage for ControlCommand {
    const CODE: u8 = 0x7D;

    fn bytes(&self) -> [u8; 7] {
        match self {
            ControlCommand::ContinuousRealTimeData => [0xA1, 0, 0, 0, 0, 0, 0],
            ControlCommand::StopRealTimeData => [0xA2, 0, 0, 0, 0, 0, 0],
            ControlCommand::AskForDeviceIdentifier => [0xAA, 0, 0, 0, 0, 0, 0],
            ControlCommand::InformDeviceConnected => [0xAF, 0, 0, 0, 0, 0, 0],
            ControlCommand::AskForStorageDataSegmentAmount(user_index) => {
                [0xA3, *user_index, 0, 0, 0, 0, 0]
            }
            ControlCommand::AskForStorageDataLength(user_index, data_segment) => {
                [0xA4, *user_index, *data_segment, 0, 0, 0, 0]
            }
            ControlCommand::AskForStorageStartTime(user_index, data_segment) => {
                [0xA5, *user_index, *data_segment, 0, 0, 0, 0]
            }
            ControlCommand::AskForStorageData(user_index, data_segment) => {
                [0xA6, *user_index, *data_segment, 0, 0, 0, 0]
            }
            ControlCommand::StopStorageData => [0xA7, 0, 0, 0, 0, 0, 0],
            ControlCommand::AskForUserInformation(user_index) => [0xAB, *user_index, 0, 0, 0, 0, 0],
            ControlCommand::AskWhetherSupportPI => [0xAC, 0, 0, 0, 0, 0, 0],
            ControlCommand::AskForUserAmount => [0xAD, 0, 0, 0, 0, 0, 0],
            ControlCommand::DeleteStorageData(user_index, data_segment) => {
                [0xAE, *user_index, *data_segment, 0, 0, 0, 0]
            }
            ControlCommand::AskForStorageDataIdentifiers => [0xB0, 0, 0, 0, 0, 0, 0],
            ControlCommand::SynchronizeDeviceTime(hour, minute, second) => {
                [0xB1, *hour, *minute, *second, 0, 0, 0]
            }
            ControlCommand::SynchronizeDeviceDate(year_high, year_low, month, day, week) => {
                [0xB2, *year_high, *year_low, *month, *day, *week, 0]
            }
            ControlCommand::AskForStorageDataIdentifiers2(user_index, data_segment) => {
                [0xA3, *user_index, *data_segment, 0, 0, 0, 0]
            }
        }
    }
}

/// Set new device identifier
pub struct SetDeviceId([u8; 7]);

impl SetDeviceId {
    /// Create new set device identifier package
    pub fn new(id: impl AsRef<[u8]>) -> Self {
        let str: [u8; 7] = id.as_ref().try_into().expect("Wrong length");
        if !str.iter().all(|c| matches!(c, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' )) {
            panic!("Invalid character")
        }
        Self(str)
    }
}

impl OutgoingPackage for SetDeviceId {
    const CODE: u8 = 0x04;

    fn bytes(&self) -> [u8; 7] {
        self.0
    }
}
