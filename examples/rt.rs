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
extern crate f3;
extern crate xbee_zigbee;
extern crate byteorder;
extern crate heapless;

use cortex_m::peripheral::ITM;
use f3::hal::prelude::*; 
use f3::hal::serial::{Event, Rx, Serial, Tx};
use f3::hal::stm32f30x::{self, USART2, TIM6};
use f3::hal::timer::{self, Timer};
use rtfm::{app, Resource, Threshold};
use xbee_zigbee::*;
use byteorder::{ByteOrder, BE};
use heapless::consts::*;
use heapless::ring_buffer::{RingBuffer, Consumer, Producer};

const INTERVAL:u32 = 10; // seconds

app! {
    device: stm32f30x,

    resources: {
        static RB: RingBuffer<u8, U1024> = RingBuffer::new();
        static RXP: Producer<'static, u8, [u8,U1024]>;
        static RXC: Consumer<u8>;
        static TX: Tx<USART2>;
        static RX: Rx<USART2>;
        static ITM: ITM;
        static TIMER: Timer<TIM6>;
        static DTO: Option<u64>;
        static TEMP: u16;
    },

    tasks: {
        USART2_EXTI26: {
            path: receive,
            priority: 2,
            resources: [RX, ITM, DTO, RXP],
        },
        TIM6_DACUNDER: {
            path: on_timer,
            priority: 1,
            resources: [ITM, TIMER, TX, DTO, TEMP]
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
        115_200.bps(),
        clocks,
        &mut rcc.apb1,
    );
    serial.listen(Event::Rxne);
    let (tx, rx) = serial.split();

    let mut timer = Timer::tim6(p.device.TIM6, INTERVAL.hz(), clocks, &mut rcc.apb1);
    timer.listen(timer::Event::TimeOut);
    iprintln!(&mut itm.stim[0], "Listening...");
    
    init::LateResources { TX: tx, RX: rx,
        ITM: itm, TIMER: timer,
        DTO: None, TEMP: 0 }
}

fn idle(_t: &mut Threshold, mut _r: idle::Resources) -> ! {
    let (mut rxc, mut itm, mut dto) = (r.RXC, r.ITM, r.DTO);
    let mut pop = || {
        match rxc.dequeue() {
            | Some(x) -> Ok(x),
            | _ -> Error(nb::WouldBlock)
        }
    }
    let r = serializer::read(&mut || , &mut |res| {
        match res {
        | frame::Inbound::RxPacket { ref data, .. } => {
            let x = BE::read_u64(&data[1..]); // Assume Data::PollCmd thunderdome packet
            *dto = Some(x)
          },
        | other =>
            iprintln!(&mut itm.stim[0], "Recived frame: {:?}", other),
        }
    });
    iprintln!(&mut itm.stim[0], "Recived: {:?}", r)

    loop {
        rtfm::wfi();
    }
}

fn on_timer(t: &mut Threshold, mut r: TIM6_DACUNDER::Resources) {
    // Clear flag to avoid getting stuck in interrupt
    r.TIMER.wait().unwrap();
    let mut tx = r.TX;
    
    r.TEMP.claim_mut(t, |temp, _t| *temp += 1);
    let mut local_dto = None;
    r.DTO.claim(t, |dto, _t| local_dto = *dto);

    match local_dto {
    | Some(dto) => {
        let mut data: [u8; 12] = [0,0,0,0,0,0,0,0,0,0,0,0];
        data[0] = 0x1; // Data::TempReading thunderdome packet
        BE::write_u64(&mut data[1..], dto);
        BE::write_u16(&mut data[10..], *r.TEMP);
        let mut frame = frame::Outbound::TxRequest {
                frame_id: (*r.TEMP & 0xFF) as u8,
                dest_addr: frame::Address::COORDINATOR,
                dest_mac: frame::MAC::COORDINATOR,
                bc_radius: 0x01,
                options: frame::TxOptions::EXTENDED_TIMEOUT,
                data: &data
            };
        serializer::write(&mut |x| tx.write(x), &mut frame).unwrap();
        r.ITM.claim_mut(t, |itm,_t| iprintln!(&mut itm.stim[0], "\nSent"));
        
        r.DTO.claim_mut(t, |global_dto, _t| *global_dto = Some(dto+(INTERVAL as u64)*1000)); // add 10s
    },
    | _ => ()
    }
}

fn receive(_t: &mut Threshold, r: USART2_EXTI26::Resources) {
    let (mut rx, mut rxp) = (r.RX, r.RXP);
    rxp.enqueue(block!(rx.read()).unwrap());
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