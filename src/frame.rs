use core::iter::ExactSizeIterator;

#[derive(Debug, PartialEq)]
pub struct Address {
    pub high: u8,
    pub low: u8,
}

impl Address {
    pub const COORDINATOR: Address = Address { high: 0, low: 0 };
    pub const UNKNOWN: Address = Address {
        high: 0xFF,
        low: 0xFE,
    };

    fn from<'a>(iter: &mut Iterator<Item = &'a u8>) -> Address {
        Address {
            high: *iter.next().unwrap(),
            low: *iter.next().unwrap(),
        }
    }

    // fn from_word(word: u16) -> Address {
    //     Address {
    //         high: (word >> 8) as u8,
    //         low: (word & 0xFF) as u8,
    //     }
    // }
}

#[derive(Debug, PartialEq)]
pub struct MAC {
    pub high: u32,
    pub low: u32,
}

impl MAC {
    pub const COORDINATOR: MAC = MAC { high: 0, low: 0 };
    pub const BROADCAST: MAC = MAC {
        high: 0x0000,
        low: 0xFFFF,
    };

    fn from<'a>(iter: &mut Iterator<Item = &'a u8>) -> MAC {
        MAC {
            high: ((*iter.next().unwrap() as u32) << 24)
                | ((*iter.next().unwrap() as u32) << 16)
                | ((*iter.next().unwrap() as u32) << 8)
                | (*iter.next().unwrap() as u32),
            low: ((*iter.next().unwrap() as u32) << 24)
                | ((*iter.next().unwrap() as u32) << 16)
                | ((*iter.next().unwrap() as u32) << 8)
                | (*iter.next().unwrap() as u32),
        }
    }

    fn at(&self, i: usize) -> Result<u8, ()> {
        match i {
            | 0 => Ok((self.high >> 24) as u8),
            | 1 => Ok(((self.high >> 16) & 0xFF) as u8),
            | 2 => Ok(((self.high >> 8) & 0xFF) as u8),
            | 3 => Ok((self.high & 0xFF) as u8),
            | 4 => Ok((self.low >> 24) as u8),
            | 5 => Ok(((self.low >> 16) & 0xFF) as u8),
            | 6 => Ok(((self.low >> 8) & 0xFF) as u8),
            | 7 => Ok((self.low & 0xFF) as u8),
            | _ => Err(()),
        }
    }
}

bitflags! {
    pub struct TxOptions: u8 {
        const DISABLE_RETRIES = 0x01;
        const ENABLE_ENCRYPTION = 0x20;
        const EXTENDED_TIMEOUT = 0x40;
    }
}

bitflags! {
    pub struct RxOptions: u8 {
        const PACKET_ACKNOWLEDGED = 0x01;
        const BROADCAST_PACKET = 0x02;
        const PACKET_ENCRYPTED = 0x20;
    }
}

/// bitfield
/// [0..2] reserved
/// [3..6] analog
/// [7..15] digital
// TODO: test if the reserved bits are on the top or bottom and what order
bitflags! {
    pub struct ChannelIndicator: u16 {
        const A3 = 0b0001000000000000;
        const A2 = 0b0000100000000000;
        const A1 = 0b0000010000000000;
        const A0 = 0b0000001000000000;
        const D8 = 0b0000000100000000;
        const D7 = 0b0000000010000000;
        const D6 = 0b0000000001000000;
        const D5 = 0b0000000000100000;
        const D4 = 0b0000000000010000;
        const D3 = 0b0000000000001000;
        const D2 = 0b0000000000000100;
        const D1 = 0b0000000000000010;
        const D0 = 0b0000000000000001;
    }
}

impl ChannelIndicator {
    fn contains_digital(&self) -> bool {
        self.contains(
            ChannelIndicator::D0
                | ChannelIndicator::D1
                | ChannelIndicator::D2
                | ChannelIndicator::D3
                | ChannelIndicator::D4
                | ChannelIndicator::D5
                | ChannelIndicator::D6
                | ChannelIndicator::D7
                | ChannelIndicator::D8,
        )
    }
}

#[derive(Debug, PartialEq)]
pub enum AtCommandStatus {
    Ok,
    Error,
    InvalidCommand,
    InvalidParam,
    TxFailure,
}

impl AtCommandStatus {
    // Done instead of using the "From" trait to keep the conversion private
    fn from(val: u8) -> Result<AtCommandStatus, u8> {
        match val {
            | 0 => Ok(AtCommandStatus::Ok),
            | 1 => Ok(AtCommandStatus::Error),
            | 2 => Ok(AtCommandStatus::InvalidCommand),
            | 3 => Ok(AtCommandStatus::InvalidParam),
            | 4 => Ok(AtCommandStatus::TxFailure),
            | x => Err(x),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum TxStatus {
    Success,
    NoAck,
    CcaFailure,
    InvalidDestination,
    NetworkAckFailure,
    NotConnected,
    SelfAddressed,
    AddressNotFound,
    RouteNotFound,
    BroadcastSourceFailed, // to hear a neighbor relay the message
    InvalidBindingTableIndex,
    ResourceError,      // lack of free buffers, timers, etc.
    AttemptedBroadcast, // with APS transmission
    AttemptedUnicast,   // with APS transmission, but EE=0
    InternalError,
    ResourceDepletion,
    PayloadTooLarge,
    IndirectMessageUnrequested,
}

impl TxStatus {
    // Done instead of using the "From" trait to keep the conversion private
    fn from(val: u8) -> Result<TxStatus, u8> {
        match val {
            | 0x00 => Ok(TxStatus::Success),
            | 0x01 => Ok(TxStatus::NoAck),
            | 0x02 => Ok(TxStatus::CcaFailure),
            | 0x15 => Ok(TxStatus::InvalidDestination),
            | 0x21 => Ok(TxStatus::NetworkAckFailure),
            | 0x22 => Ok(TxStatus::NotConnected),
            | 0x23 => Ok(TxStatus::SelfAddressed),
            | 0x24 => Ok(TxStatus::AddressNotFound),
            | 0x25 => Ok(TxStatus::RouteNotFound),
            | 0x26 => Ok(TxStatus::BroadcastSourceFailed),
            | 0x2B => Ok(TxStatus::InvalidBindingTableIndex),
            | 0x2C => Ok(TxStatus::ResourceError),
            | 0x2E => Ok(TxStatus::AttemptedUnicast),
            | 0x2D => Ok(TxStatus::AttemptedBroadcast),
            | 0x31 => Ok(TxStatus::InternalError),
            | 0x32 => Ok(TxStatus::ResourceDepletion),
            | 0x74 => Ok(TxStatus::PayloadTooLarge),
            | 0x75 => Ok(TxStatus::IndirectMessageUnrequested),
            | x => Err(x),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum ModemStatus {
    HardwareReset,
    WatchdogReset,
    JoinedNetwork,
    Dissociated,
    CoordinatorStarted,
    NetworkSecurityUpdated,
    ModemConfigurationChanged, // while join in progress
    EmberZigbeeStackError,
    InputVoltageTooHigh,
}

impl ModemStatus {
    fn from(val: u8) -> Result<ModemStatus, u8> {
        match val {
            | 0x00 => Ok(ModemStatus::HardwareReset),
            | 0x01 => Ok(ModemStatus::WatchdogReset),
            | 0x02 => Ok(ModemStatus::JoinedNetwork),
            | 0x03 => Ok(ModemStatus::Dissociated),
            | 0x06 => Ok(ModemStatus::CoordinatorStarted),
            | 0x07 => Ok(ModemStatus::NetworkSecurityUpdated),
            | 0x11 => Ok(ModemStatus::ModemConfigurationChanged), // while join in progress
            | 0x80 => Ok(ModemStatus::EmberZigbeeStackError),
            | 0x0D => Ok(ModemStatus::InputVoltageTooHigh),
            | x => Err(x),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum DiscoStatus {
    NoDiscoveryOverhead,
    AddressDiscovery,
    RouteDiscovery,
    AddressAndRoute,
    ExtendedTimeoutDiscovery,
}

impl DiscoStatus {
    fn from(val: u8) -> Result<DiscoStatus, u8> {
        match val {
            | 0x00 => Ok(DiscoStatus::NoDiscoveryOverhead),
            | 0x01 => Ok(DiscoStatus::AddressDiscovery),
            | 0x02 => Ok(DiscoStatus::RouteDiscovery),
            | 0x03 => Ok(DiscoStatus::AddressAndRoute),
            | 0x40 => Ok(DiscoStatus::ExtendedTimeoutDiscovery),
            | x => Err(x),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Outbound<'a> {
    TxRequest {
        frame_id: u8,
        dest_mac: MAC,
        dest_addr: Address,
        bc_radius: u8,
        options: TxOptions,
        data: &'a [u8],
    },
    // AtCommand {
    //     frame_id: u8,
    //     at_cmd: [u8; 2],
    //     params: &'a [u8],
    // },
    // AtCommandQueueParam {
    //     frame_id: u8,
    //     at_cmd: [u8; 2],
    //     params: &'a [u8],
    // },
    // RemoteAtCommand {
    //     frame_id: u8,
    //     dest_mac: MAC,
    //     dest_addr: Address,
    //     at_cmd: [u8; 2],
    //     params: &'a [u8],
    // }
}

#[derive(Debug, PartialEq)]
pub enum Inbound<'a> {
    RxPacket {
        source_mac: MAC,
        source_addr: Address,
        options: RxOptions,
        data: &'a [u8],
    },
    AtCommandResponse {
        frame_id: u8,
        at_cmd: [u8; 2],
        status: AtCommandStatus,
        data: &'a [u8],
    },
    TransmitStatus {
        frame_id: u8,
        dest_addr: Address,
        txr_count: u8,
        status: TxStatus,
        disco_status: DiscoStatus,
    },
    ModemStatus {
        status: ModemStatus,
    },
    RemoteAtCommandResponse {
        frame_id: u8,
        source_mac: MAC,
        source_addr: Address,
        at_cmd: [u8; 2],
        status: AtCommandStatus,
        data: &'a [u8],
    },
}

pub struct OutboundIterator<'a> {
    offset: u8,
    frame: &'a Outbound<'a>,
}

impl<'a> Outbound<'a> {
    pub fn to_iter<'b>(&'b mut self) -> OutboundIterator<'b> {
        OutboundIterator {
            offset: 0,
            frame: self,
        }
    }
}

impl<'a> Iterator for OutboundIterator<'a> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        let res = match self.frame {
            | &Outbound::TxRequest {
                ref frame_id,
                ref dest_mac,
                ref dest_addr,
                ref bc_radius,
                ref options,
                ref data,
            } => match self.offset as usize {
                | 0 => Some(0x10),
                | 1 => Some(*frame_id),
                | n if n >= 2 && n <= 9 => dest_mac.at(n - 2).ok(),
                | 10 => Some(dest_addr.high),
                | 11 => Some(dest_addr.low),
                | 12 => Some(*bc_radius),
                | 13 => Some(options.bits()),
                | n if n >= 14 && n < data.len()+14 => Some(data[n - 14]),
                | _ => None,
            },
            // | Outbound::AtCommand {frame_id, at_cmd, params} => Some(0x08),
            // | Outbound::AtCommandQueueParam atParam => Some(0x09),
            // | Outbound::RemoteAtCommand remoteAt => Some(0x17),
        };
        self.offset = self.offset + 1;
        res
    }
}

impl<'a> ExactSizeIterator for OutboundIterator<'a> {
    fn len(&self) -> usize {
        match self.frame {
            | &Outbound::TxRequest { data, .. } => 14 + data.len(),
        }
    }
}

impl<'a> Inbound<'a> {
    pub fn parse(data: &[u8]) -> Result<Inbound, u8> {
        let len = data.len();
        let mut iter = data.iter();
        match *iter.next().unwrap() {
            | 0x90 if len > 10 => Ok(Inbound::RxPacket {
                source_mac: MAC::from(&mut iter),
                source_addr: Address::from(&mut iter),
                options: RxOptions::from_bits_truncate(*iter.next().unwrap()),
                data: iter.as_slice(),
            }),
            | 0x8B => Ok(Inbound::TransmitStatus {
                frame_id: *iter.next().unwrap(),
                dest_addr: Address::from(&mut iter),
                txr_count: *iter.next().unwrap(),
                status: TxStatus::from(*iter.next().unwrap()).unwrap(),
                disco_status: DiscoStatus::from(*iter.next().unwrap()).unwrap(),
            }),
            | n => Err(n),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    // #[test]
    // fn at_commmand_response_bd_parse_test() {
    //     let unpacked_data = [0x88, 0x01, 0x42, 0x44, 0x00];
    //     let parsed_data = Inbound::parse(&unpacked_data[..]).unwrap();

    //     let test_data = Inbound::AtCommandResponse {
    //         frame_id: 0x01,
    //         at_cmd: [b'B', b'D'],
    //         status: AtCommandStatus::Ok,
    //         data: &[],
    //     };

    //     assert_eq!(parsed_data, test_data);
    // }

    #[test]
    fn tx_status_parse_test() {
        let unpacked_data = [0x8B, 0x01, 0x42, 0x43, 0x02, 0x23, 0x01];
        let parsed_data = Inbound::parse(&unpacked_data).unwrap();

        let test_data = Inbound::TransmitStatus {
            frame_id: 0x01,
            dest_addr: Address {
                high: 0x42,
                low: 0x43,
            },
            txr_count: 0x02,
            status: TxStatus::SelfAddressed,
            disco_status: DiscoStatus::AddressDiscovery,
        };

        assert_eq!(parsed_data, test_data);
    }

    #[test]
    fn rx_parse_test() {
        let unpacked_data = [
            0x90, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x22, 0x42, 0x43,
        ];
        let parsed_data = Inbound::parse(&unpacked_data).unwrap();

        let test_data = Inbound::RxPacket {
            source_mac: MAC {
                high: 0x01020304,
                low: 0x05060708,
            },
            source_addr: Address {
                high: 0x09,
                low: 0x0A,
            },
            options: RxOptions::BROADCAST_PACKET | RxOptions::PACKET_ENCRYPTED,
            data: &[0x42, 0x43],
        };

        assert_eq!(parsed_data, test_data);
    }

    // #[test]
    // fn modem_status_parse_test() {
    //     let unpacked_data = [0x8A, 0x00];
    //     let parsed_data = Inbound::parse(&unpacked_data[..]).unwrap();

    //     let test_data = Inbound::ModemStatus {
    //         status: ModemStatus::HardwareReset,
    //     };

    //     assert_eq!(parsed_data, test_data);
    // }
}
