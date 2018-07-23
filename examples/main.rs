//! Lora interface 
//!
//! This is a blocking version, here the processor busy waits while processing messages.
//#![deny(unsafe_code)]
//#![deny(warnings)]
#![no_std]
// #![no_main]

#[macro_use]
extern crate nb;
extern crate f3;
extern crate xbee_zigbee;
extern crate byteorder;
#[macro_use]
extern crate cortex_m;
extern crate heapless;
// #[macro_use(entry, exception)]
extern crate cortex_m_rt as rt;
// use rt::ExceptionFrame;

// entry!(main);

use byteorder::{ByteOrder, BE};
use cortex_m::asm;
use f3::hal::prelude::*;
use f3::hal::serial::{Serial};
use f3::hal::stm32f30x;
use xbee_zigbee::*;
use heapless::*;

fn main() {
    let p = stm32f30x::Peripherals::take().unwrap();
    let mut flash = p.FLASH.constrain();
    let mut rcc = p.RCC.constrain();
    let mut gpioa = p.GPIOA.split(&mut rcc.ahb);
    let mut gpiob = p.GPIOB.split(&mut rcc.ahb);

    let clocks = rcc.cfgr.sysclk(32.mhz()).pclk1(16.mhz()).pclk2(16.mhz()).freeze(&mut flash.acr);


    let tx = gpioa.pa2.into_af7(&mut gpioa.moder, &mut gpioa.afrl);
    let rx = gpioa.pa3.into_af7(&mut gpioa.moder, &mut gpioa.afrl);
    // p.USART2.cr2.write(|w| w.linen().clear_bit().clken().clear_bit());
    // p.USART2.cr3.write(|w| w.scen().clear_bit().hdsel().clear_bit().iren().clear_bit());
    let serial = Serial::usart2(
        p.USART2,
        (tx, rx),
        115_200.bps(),
        clocks,
        &mut rcc.apb1,
    );
    let (mut tx, mut rx) = serial.split();

    let p = cortex_m::Peripherals::take().unwrap();
    util::enable_itm(&p, clocks.sysclk().0);
    let mut itm = p.ITM;

    // iprintln!(&mut itm.stim[0], "Recieved:{:?}", rx.read());
    
    // let mut frame = frame::Outbound::TxRequest {
    //         frame_id: 0x01,
    //         dest_addr: frame::Address::COORDINATOR,
    //         dest_mac: frame::MAC::COORDINATOR,
    //         bc_radius: 0x01,
    //         options: frame::TxOptions::EXTENDED_TIMEOUT,
    //         data: &[0x1,0x42]
    //     };

    // // let r = serializer::write(&mut |x| tx.write(x), &mut frame);
    
    // iprintln!(&mut itm.stim[0], "Sent: {:?}", tx.write(0x42));

    // iprintln!(&mut itm.stim[0], "Recieved:{:?}", rx.read());
    // iprintln!(&mut itm.stim[0], "Recieved:{:?}", rx.read());

    // iprintln!(&mut itm.stim[0], "Sent: {:?}", tx.write(0x43));

    // iprintln!(&mut itm.stim[0], "Recieved:{:?}", rx.read());
    // iprintln!(&mut itm.stim[0], "Recieved:{:?}", rx.read());

    // let r = serializer::read(&mut || rx.read(), &mut |res| {
    //     match res {
    //     | frame::Inbound::RxPacket { ref data, .. } => {
    //         let x = BE::read_u64(&data[1..]); // Assume Data::PollCmd thunderdome packet
    //         iprintln!(&mut itm.stim[0], "Recieved PollCmd:{:?}", x);
    //       },
    //     | other =>
    //         iprintln!(&mut itm.stim[0], "Recived frame: {:?}", other),
    //     }
    // });

    let mut buffer: Vec<u8, consts::U256> = Vec::new();
    while let Ok(r) = buffer.push(block!(rx.read()).unwrap()) {
        ;
    }
    iprintln!(&mut itm.stim[0], "Read: {:?}", buffer);

    asm::bkpt();
    loop {}
}

// exception!(HardFault, hard_fault);

// fn hard_fault(ef: &ExceptionFrame) -> ! {
//     panic!("{:#?}", ef);
// }

// exception!(*, default_handler);

// fn default_handler(irqn: i16) {
//     panic!("Unhandled exception (IRQn = {})", irqn);
// }

pub mod util {
    use cortex_m;
    use core::ptr;

    // enable ITM
    // TODO check for API in the cortex-m crate to do this (https://github.com/japaric/cortex-m/issues/82)
    pub fn enable_itm(p : &cortex_m::Peripherals, clocks : u32) {
        unsafe {
            // enable TPIU and ITM
            p.DCB.demcr.modify(|r| r | (1 << 24));

            // prescaler
            let swo_freq = 2_000_000;
            p.TPIU.acpr.write((clocks / swo_freq) - 1);

            // SWO NRZ
            p.TPIU.sppr.write(2);

            p.TPIU.ffcr.modify(|r| r & !(1 << 1));

            // STM32 specific: enable tracing in the DBGMCU_CR register
            const DBGMCU_CR: *mut u32 = 0xe0042004 as *mut u32;
            let r = ptr::read_volatile(DBGMCU_CR);
            ptr::write_volatile(DBGMCU_CR, r | (1 << 5));

            // unlock the ITM
            p.ITM.lar.write(0xC5ACCE55);

            p.ITM.tcr.write(
                (0b000001 << 16) | // TraceBusID
                (1 << 3) | // enable SWO output
                (1 << 0), // enable the ITM
            );

            // enable stimulus port 0
            p.ITM.ter[0].write(1);
        }
    }
}