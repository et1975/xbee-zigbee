use frame;
use heapless::*;
use nb;

macro_rules! block {
    ($e:expr, $err:expr) => {
        loop {
            #[allow(unreachable_patterns)]
            match $e {
                Err(nb::Error::Other(e)) => {
                    #[allow(unreachable_code)]
                    break Err($err(e))
                }
                Err(nb::Error::WouldBlock) => {}
                Ok(x) => break Ok(x),
            }
        }
    };
}

const START: u8 = 0x7E;
// const ESCAPE: u8 = 0x7D;
// const XON: u8 = 0x11;
// const XOFF: u8 = 0x13;

#[derive(Debug)]
pub enum SerializationError<E> {
    WouldOverflow,
    Other(E),
}

impl<E> From<u8> for SerializationError<E> {
    fn from(_e: u8) -> SerializationError<E> {
        SerializationError::WouldOverflow
    }
}

enum SerializationState {
    Start,
    LenH,
    LenL,
    Data,
    //Checksum,
    Done,
}

struct FrameSerializer<I> {
    state: SerializationState,
    data: I,
    checksum: u8,
}

impl<I> FrameSerializer<I>
where
    I: ExactSizeIterator<Item = u8>,
{
    pub fn new(data: I) -> FrameSerializer<I> {
        FrameSerializer {
            state: SerializationState::Start,
            data: data,
            checksum: 0,
        }
    }
}

impl<I> Iterator for FrameSerializer<I>
where
    I: ExactSizeIterator<Item = u8>,
{
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        match self.state {
            | SerializationState::Start => {
                self.state = SerializationState::LenH;
                Some(START)
            }
            | SerializationState::LenH => {
                self.state = SerializationState::LenL;
                Some((self.data.len() >> 8) as u8)
            }
            | SerializationState::LenL => {
                self.state = SerializationState::Data;
                Some(self.data.len() as u8)
            }
            | SerializationState::Data => {
                if let Some(val) = self.data.next() {
                    self.checksum = self.checksum.wrapping_add(val);
                    Some(val)
                } else {
                    self.state = SerializationState::Done;
                    Some(0xff - self.checksum)
                }
            }
            | SerializationState::Done => None,
        }
    }
}

pub fn write<E, TX: FnMut(u8) -> nb::Result<(), E>>(
    tx: &mut TX,
    frame: &mut frame::Outbound,
) -> Result<(), SerializationError<E>> {
    let mut buffer: Vec<u8, consts::U256> = Vec::new();
    let fs = FrameSerializer::new(frame.to_iter());
    for b in fs {
        buffer.push(b)?
    }

    for b in buffer.iter() {
        block!(tx(*b), SerializationError::Other)?
    }

    Ok(())
}

#[derive(Debug)]
pub enum DeserializationError<E> {
    NoStart,
    WouldOverflow,
    Unsupported(u8),
    BadChecksum(u8),
    Other(E),
}

impl<E> From<u8> for DeserializationError<E> {
    fn from(_e: u8) -> DeserializationError<E> {
        DeserializationError::WouldOverflow
    }
}

pub fn read<E, RX: FnMut() -> nb::Result<u8, E>, C: FnMut(frame::Inbound) -> ()>(
    rx: &mut RX,
    cont: &mut C,
) -> Result<(), DeserializationError<E>> {
    let mut buffer: Vec<u8, consts::U256> = Vec::new();
    let len = 
        match block!(rx(), DeserializationError::Other)? {
            | byte if byte == START => {
                // length
                let lenh = block!(rx(), DeserializationError::Other)?;
                let lenl = block!(rx(), DeserializationError::Other)?;
                ((lenh as u16) << 8 | (lenl as u16)) as usize
            },
            | _ => return Err(DeserializationError::NoStart)
        };
    while buffer.len() <= len {
        let byte = block!(rx(), DeserializationError::Other)?;
        buffer.push(byte)?;
    }
    Ok(cont(unpack(&buffer)?))
}

fn unpack<E>(buf: &[u8]) -> Result<frame::Inbound, DeserializationError<E>> {
    let (checksum, data) = buf.split_last().unwrap();
    let check = data.iter().fold(0, |acc: u8, &val| acc.wrapping_add(val));
    if checksum.wrapping_add(check) != 0xFF {
        return Err(DeserializationError::BadChecksum(check));
    }

    let frame = frame::Inbound::parse(data);
    match frame {
        | Ok(frame) => Ok(frame),
        | Err(n) => Err(DeserializationError::Unsupported(n)),
    }
}

#[cfg(test)]
mod test {
    use frame::*;
    use heapless::*;
    use serializer::*;
    use nb;

    #[test]
    fn write_test() {
        let mut frame = Outbound::TxRequest {
            frame_id: 0x01,
            dest_addr: Address::UNKNOWN,
            dest_mac: MAC { 
                high: 0x0013A200,
                low: 0x400A0127
            },
            bc_radius: 0x00,
            options: TxOptions::EXTENDED_TIMEOUT,
            data: &[0x54, 0x78, 0x32, 0x43, 0x6F, 0x6F, 0x72, 0x64]
        };
        let test_data = [
            0x7E, 0x00, 0x16, 0x10, 0x01, 0x00, 0x13, 0xA2, 0x00, 0x40, 0x0A, 0x01, 0x27, 0xFF,
            0xFE, 0x00, 0x40, 0x54, 0x78, 0x32, 0x43, 0x6F, 0x6F, 0x72, 0x64, 0x95
        ];
        let mut res : Vec<u8, consts::U256> = Vec::new();
        write(&mut |x| res.push(x).or(Err(nb::Error::Other(x))), &mut frame).unwrap();

        assert_eq!(res, test_data);
    }

    #[test]
    fn read_test() {
        let mut i = [
            0x7E, 0x00, 0x07, 0x8B, 0x01, 0x7D, 0x84, 0x00, 0x00, 0x01, 0x71,
        ].iter();
        let test_data = Inbound::TransmitStatus {
            frame_id: 0x01,
            dest_addr: Address {
                high: 0x7D,
                low: 0x84,
            },
            txr_count: 0x00,
            status: TxStatus::Success,
            disco_status: DiscoStatus::AddressDiscovery,
        };

        read(&mut || i.next().map(|x| *x).ok_or(nb::Error::Other(())), &mut |frame| {
            assert_eq!(frame, test_data);
        }).unwrap();
    }
}
