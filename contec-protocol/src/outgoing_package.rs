use crate::pulox::OutgoingPackage;

pub enum ControlCommand {
    ContinuousRealTimeData,
    StopRealTimeData,
    InformDeviceConnected,
    AskForDeviceIdentifier,
}

impl OutgoingPackage for ControlCommand {
    const CODE: u8 = 0x7D;

    fn bytes(&self) -> [u8; 7] {
        match self {
            ControlCommand::ContinuousRealTimeData => [0xA1, 0, 0, 0, 0, 0, 0],
            ControlCommand::StopRealTimeData => [0xA2, 0, 0, 0, 0, 0, 0],
            ControlCommand::AskForDeviceIdentifier => [0xAA, 0, 0, 0, 0, 0, 0],
            ControlCommand::InformDeviceConnected => [0xAF, 0, 0, 0, 0, 0, 0],
        }
    }
}

pub struct SetDeviceId([u8;7]);

impl SetDeviceId {
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


