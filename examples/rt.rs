//! Lora interface (rtfm version)
//!
//! Here the processor sleeps most of the time and only wakes up to sent and process messages.
//!
//! This example uses the [Real Time For the Masses framework](https://docs.rs/cortex-m-rtfm/~0.3)
// #![deny(unsafe_code)]
#![deny(warnings)]
#![feature(proc_macro)]
#![no_std]

#[macro_use]
extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate cortex_m_rtfm as rtfm;
extern crate stm32f30x_hal as hal;
extern crate xbee_zigbee;
extern crate byteorder;
extern crate heapless;

use cortex_m::peripheral::ITM;
use hal::prelude::*; 
use hal::serial::{Event, Rx, Serial, Tx};
use hal::stm32f30x::{self, USART2, TIM6};
use hal::timer::{self, Timer};
use rtfm::{app, Resource, Threshold};
use xbee_zigbee::*;
use byteorder::{ByteOrder, BE};

const INTERVAL:u32 = 10; // seconds

app! {
    device: stm32f30x,

    resources: {
        static COUNTER: u32;
        static TX: Tx<USART2>;
        static RX: Rx<USART2>;
        static ITM: ITM;
        static TIMER: Timer<TIM6>;
        static DTO: Option<i64>;
        static TEMP: u16;
    },

    tasks: {
        USART2_EXTI26: {
            path: receive,
            priority: 2,
            resources: [RX, ITM, DTO],
        },
        TIM6_DACUNDER: {
            path: on_timer,
            priority: 1,
            resources: [ITM, TIMER, TX, DTO, TEMP, COUNTER]
        }
    },

    idle: {
        resources: [ITM]
    },
}

fn init(p: init::Peripherals) -> init::LateResources {
    let mut flash = p.device.FLASH.constrain();
    let mut rcc = p.device.RCC.constrain();
    let mut gpioa = p.device.GPIOA.split(&mut rcc.ahb);

    let clocks = rcc.cfgr.sysclk(32.mhz()).pclk1(16.mhz()).freeze(&mut flash.acr);

    util::enable_itm(&p.core, clocks.sysclk().0);
    let mut itm = p.core.ITM;

    let tx = gpioa.pa2.into_af7(&mut gpioa.moder, &mut gpioa.afrl);
    let rx = gpioa.pa3.into_af7(&mut gpioa.moder, &mut gpioa.afrl);

    let mut serial = Serial::usart2(
        p.device.USART2,
        (tx, rx),
        9600.bps(),
        clocks,
        &mut rcc.apb1,
    );
    serial.listen(Event::Rxne);
    let (tx, rx) = serial.split();

    let mut timer = Timer::tim6(p.device.TIM6, 1.hz(), clocks, &mut rcc.apb1);
    timer.listen(timer::Event::TimeOut);
    iprintln!(&mut itm.stim[0], "Listening...");
    
    init::LateResources { COUNTER: 0,
        TX: tx, RX: rx,
        ITM: itm, TIMER: timer,
        DTO: None, TEMP: 0 }
}

fn idle(_t: &mut Threshold, mut _r: idle::Resources) -> ! {
    loop {
        rtfm::wfi();
    }
}

fn on_timer(t: &mut Threshold, mut r: TIM6_DACUNDER::Resources) {
    // Clear flag to avoid getting stuck in interrupt
    r.TIMER.wait().unwrap();
    let (mut counter, mut tx) = (r.COUNTER, r.TX);
    *counter += 1;

    r.TEMP.claim_mut(t, |temp, _t| *temp += 1);
    let mut local_dto = None;
    r.DTO.claim_mut(t, |dto, _t| {
        local_dto = *dto;
        *dto = dto.map(|v| v+1000); // add 1s
    });

    match (*counter % INTERVAL, local_dto) {
    | (0,Some(dto)) => {
        let mut data: [u8; 11] = [0,0,0,0,0,0,0,0,0,0,0];
        data[0] = 0x1; // Data::TempReading thunderdome packet
        BE::write_i64(&mut data[1..], dto);
        BE::write_u16(&mut data[9..], *r.TEMP);
        let mut frame = frame::Outbound::TxRequest {
                frame_id: (*counter & 0xFF) as u8,
                dest_addr: frame::Address::COORDINATOR,
                dest_mac: frame::MAC::COORDINATOR,
                bc_radius: 0x01,
                options: frame::TxOptions::EXTENDED_TIMEOUT,
                data: &data
            };
        serializer::write(&mut |x| tx.write(x), &mut frame).unwrap();
//        r.ITM.claim_mut(t, |itm,_t| iprintln!(&mut itm.stim[0], "Sent: {:?}", data));
    },
    | _ => ()
    }
}

fn receive(_t: &mut Threshold, r: USART2_EXTI26::Resources) {
    let (mut rx, mut itm, mut dto) = (r.RX, r.ITM, r.DTO);
    if rx.has_data() {
        serializer::read(&mut || rx.read(), &mut |res| {
            match res {
            | frame::Inbound::RxPacket { ref data, .. } => {
                let x = BE::read_i64(&data[1..]); // Assume Data::PollCmd thunderdome packet
                *dto = Some(x)
            },
            | _ => ()
            }
            iprintln!(&mut itm.stim[0], "Recived frame: {:?}", res)
        }).unwrap();
    }
    rx.clear_overrun_error();
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