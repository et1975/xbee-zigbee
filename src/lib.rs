#![no_std]

#[macro_use]
extern crate bitflags;
extern crate embedded_hal;
#[macro_use]
extern crate nb;
extern crate heapless;

pub mod frame;
pub mod serializer;

use embedded_hal::blocking::delay::DelayMs;
use embedded_hal::blocking::serial::Write as BlockingWrite;
use embedded_hal::serial::{Read, Write};

pub struct XBeeTransparent<'a, 'b, U: 'a, D: 'b> {
    serial: &'a mut U,
    timer: &'b mut D,
    cmd_char: u8,
    guard_time: u16,
}

trait XBeeApi {
    type Error;

    fn send(&mut self, &frame::Outbound) -> Result<(), Self::Error>;
    fn receive(&mut self) -> Result<frame::Inbound, Self::Error>;
}

pub struct XBeeApiUart<'a, U: 'a> {
    serial: &'a mut U,
}

impl<'a, 'b, E, U, D> XBeeTransparent<'a, 'b, U, D>
where
    U: Read<u8, Error = E> + BlockingWrite<u8, Error = E>,
    D: DelayMs<u16>,
{
    pub fn new(
        uart: &'a mut U,
        delay: &'b mut D,
        cmd_char: u8,
        guard_time: u16,
    ) -> XBeeTransparent<'a, 'b, U, D> {
        XBeeTransparent {
            serial: uart,
            timer: delay,
            cmd_char,
            guard_time,
        }
    }

    pub fn enter_command_mode(&mut self) -> Result<(), E> {
        // wait for guard time
        self.timer.delay_ms(self.guard_time);
        // send command character x3
        self.serial.bwrite_all(&[self.cmd_char; 3])?;
        // wait for "OK"
        loop {
            match self.serial.read() {
                Ok(b'O') => break,
                Ok(_) => panic!("Got other character while waiting for OK"), // TODO: error
                Err(nb::Error::WouldBlock) => {} // keep blocking
                Err(_) => panic!("Some error while waiting for OK"), // return Err(e.into()),
            }
        }
        loop {
            match self.serial.read() {
                Ok(b'K') => break,
                Ok(_) => panic!("Got other character while waiting for OK"), // TODO: error
                Err(nb::Error::WouldBlock) => {} // keep blocking
                Err(_) => panic!("Some error while waiting for OK"), // return Err(e.into()),
            }
        }
        Ok(())
    }

    pub fn to_api(self) -> XBeeApiUart<'a, U> {
        // TODO: AT command
        XBeeApiUart::new(self.serial)
    }
}

impl<'a, 'b, U, D> Read<u8> for XBeeTransparent<'a, 'b, U, D>
where
    U: Read<u8>,
{
    type Error = U::Error;

    fn read(&mut self) -> nb::Result<u8, Self::Error> {
        self.serial.read()
    }
}

impl<'a, 'b, U, D> Write<u8> for XBeeTransparent<'a, 'b, U, D>
where
    U: Write<u8>,
{
    type Error = U::Error;

    fn write(&mut self, word: u8) -> nb::Result<(), Self::Error> {
        self.serial.write(word)
    }

    fn flush(&mut self) -> nb::Result<(), Self::Error> {
        self.serial.flush()
    }
}

impl<'a, E, U> XBeeApiUart<'a, U>
where
    U: Read<u8, Error = E> + BlockingWrite<u8, Error = E>,
{
    pub fn new(uart: &'a mut U) -> XBeeApiUart<'a, U> {
        // TODO: check that we are in API mode and if not, switch
        XBeeApiUart{ serial: uart }
    }

    // TODO: set correct size for delay
    pub fn to_transpartent<'b, D>(
        self,
        delay: &'b mut D,
        cmd_char: u8,
        guard_time: u16,
    ) -> XBeeTransparent<'a, 'b, U, D>
    where
        D: DelayMs<u16>,
    {
        // TODO: AT command
        XBeeTransparent::new(self.serial, delay, cmd_char, guard_time)
    }
}

// impl<'a, E, U> XBeeApi for XBeeApiUart<'a, U>
// where
//     U: Read<u8, Error = E> + BlockingWrite<u8, Error = E> + Write<u8, Error = E>,
// {
//     type Error = serializer::SerializationError<E>;

//     fn send(&mut self, frame : &frame::Outbound) -> Result<(), Self::Error> {
//         serializer::serialize(&mut |b| self.serial.write(b), frame)?;
//         Ok(())
//     }

//     fn receive(&mut self) -> Result<frame::Inbound, E> {
//     }
// }
