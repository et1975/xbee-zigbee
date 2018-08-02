//! Lora interface 
//!
//! This is a blocking version, here the processor busy waits while processing messages.
//#![deny(unsafe_code)]
//#![deny(warnings)]
#![no_std]
// #![no_main]

extern crate nb;
extern crate stm32f30x_hal as hal;
extern crate xbee_zigbee;
extern crate byteorder;
#[macro_use]
extern crate cortex_m;
extern crate heapless;

use hal::prelude::*;
use hal::serial::{Serial};
use hal::stm32f30x;
use xbee_zigbee::*;
use hal::gpio::{Output, PushPull};
use hal::gpio::gpioa::{PA5};
use hal::delay::Delay;

fn main() {
    let p = stm32f30x::Peripherals::take().unwrap();
    let mut flash = p.FLASH.constrain();
    let mut rcc = p.RCC.constrain();
    let mut gpioa = p.GPIOA.split(&mut rcc.ahb);

    let clocks = rcc.cfgr.freeze(&mut flash.acr);//.sysclk(32.mhz()).pclk1(16.mhz()).pclk2(16.mhz())

    let mut led : PA5<Output<PushPull>> = gpioa.pa5.into_push_pull_output(&mut gpioa.moder, &mut gpioa.otyper);

    let tx = gpioa.pa2.into_af7(&mut gpioa.moder, &mut gpioa.afrl);
    let rx = gpioa.pa3.into_af7(&mut gpioa.moder, &mut gpioa.afrl);
    // p.USART2.cr2.write(|w| w.linen().clear_bit().clken().clear_bit());
    // p.USART2.cr3.write(|w| w.scen().clear_bit().hdsel().clear_bit().iren().clear_bit());
    let serial = Serial::usart2(
        p.USART2,
        (tx, rx),
        9600.bps(),
        clocks,
        &mut rcc.apb1,
    );
    let (mut tx, mut rx) = serial.split();

    let mut p = cortex_m::Peripherals::take().unwrap();
    let mut itm = util::enable_itm(&p.DCB, &p.TPIU, &mut p.ITM, clocks.sysclk().0);
    let mut delay = Delay::new(p.SYST, clocks);

    let mut counter : u8 = 0;
    loop {
        rx.clear_overrun_error();
        led.set_high();
        let r = serializer::read(&mut || rx.read(), &mut |res| {
            led.set_low();
            match res {
            | frame::Inbound::RxPacket { ref data, .. } => {
                counter += 1;
                let mut frame = frame::Outbound::TxRequest {
                    frame_id: counter,
                    dest_addr: frame::Address::COORDINATOR,
                    dest_mac: frame::MAC::COORDINATOR,
                    bc_radius: 0x10,
                    options: frame::TxOptions::EXTENDED_TIMEOUT,
                    data: &data
                };
                delay.delay_us(100_u16);
                serializer::write(&mut |x| tx.write(x), &mut frame).unwrap();
            },
            | _ => ()
            }
            iprintln!(&mut itm.stim[0], "Recived frame: {:?}", res);
        });
        iprintln!(&mut itm.stim[0], "Read: {:?}", r);
    }
}

pub mod util {
    use cortex_m;
    use core::ptr;

    // enable ITM
    // TODO check for API in the cortex-m crate to do this (https://github.com/japaric/cortex-m/issues/82)
    pub fn enable_itm<'a>(dcb : &cortex_m::peripheral::DCB, 
                      tpiu : &cortex_m::peripheral::TPIU, 
                      itm : &'a mut cortex_m::peripheral::ITM,
                      clocks : u32) -> &'a mut cortex_m::peripheral::ITM {
        unsafe {
            // enable TPIU and ITM
            dcb.demcr.modify(|r| r | (1 << 24));

            // prescaler
            let swo_freq = 2_000_000;
            tpiu.acpr.write((clocks / swo_freq) - 1);

            // SWO NRZ
            tpiu.sppr.write(2);

            tpiu.ffcr.modify(|r| r & !(1 << 1));

            // STM32 specific: enable tracing in the DBGMCU_CR register
            const DBGMCU_CR: *mut u32 = 0xe0042004 as *mut u32;
            let r = ptr::read_volatile(DBGMCU_CR);
            ptr::write_volatile(DBGMCU_CR, r | (1 << 5));

            // unlock the ITM
            itm.lar.write(0xC5ACCE55);

            itm.tcr.write(
                (0b000001 << 16) | // TraceBusID
                (1 << 3) | // enable SWO output
                (1 << 0), // enable the ITM
            );

            // enable stimulus port 0
            itm.ter[0].write(1);

            itm
        }
    }
}