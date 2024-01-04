use crate::limero::{Emitter, Handler, Sink, Source};
use alloc::boxed::Box;
use alloc::{rc::Rc, vec::Vec};
use core::{cell::RefCell, fmt::Write, panic::PanicInfo};
use critical_section::Mutex;
use embassy_futures::select::select;
use embassy_futures::select::Either;
use embassy_futures::{select, yield_now};
use embassy_sync::{
    blocking_mutex::raw::{CriticalSectionRawMutex, NoopRawMutex},
    channel::Channel,
    signal::Signal,
};
use esp32_hal::{
    interrupt,
    peripherals::{self, UART0},
    prelude::*,
    timer::Timer,
    uart::{config::AtCmdConfig, UartRx, UartTx},
    Uart, IO,
};
use nb::block;

pub enum SerialCmd {
    SendBytes(Vec<u8>),
}
#[derive(Clone)]
pub enum SerialEvent {
    RecvBytes(Vec<u8>),
}

static RXD_CHANNEL: Channel<CriticalSectionRawMutex, SerialEvent, 3> = Channel::new();
static TXD_CHANNEL: Channel<CriticalSectionRawMutex, SerialCmd, 3> = Channel::new();
static SERIAL: Mutex<RefCell<Option<Uart<UART0>>>> = Mutex::new(RefCell::new(None));

pub struct Serial {
    rxd_channel: &'static Channel<CriticalSectionRawMutex, SerialEvent, 3>,
    txd_channel: &'static Channel<CriticalSectionRawMutex, SerialCmd, 3>,
    emitter: Rc<RefCell<Emitter<SerialEvent>>>,
}

impl Serial {
    pub fn new(mut uart0: Uart<'static, UART0>) -> Self {
        uart0
            .set_rx_fifo_full_threshold(READ_BUF_SIZE as u16)
            .unwrap();
        uart0.set_rx_fifo_full_threshold(30).unwrap();
        uart0.listen_at_cmd();
        uart0.listen_rx_fifo_full();
        interrupt::enable(
            peripherals::Interrupt::UART0,
            interrupt::Priority::Priority2,
        )
        .unwrap();

        critical_section::with(|cs| SERIAL.borrow_ref_mut(cs).replace(uart0));
        // let (tx, rx) = uart0.split();
        Self {
            rxd_channel: &RXD_CHANNEL,
            txd_channel: &TXD_CHANNEL,
            emitter: Rc::new(RefCell::new(Emitter::new())),
        }
    }

    pub async fn run(&mut self) {
        loop {
            let msg = select(self.rxd_channel.receive(), self.txd_channel.receive()).await;

            match msg {
                Either::First(rxd) => {
                    self.emitter.borrow().emit(rxd);
                }
                Either::Second(txd) => match txd {
                    SerialCmd::SendBytes(buf) => {
                        critical_section::with(|cs| {
                            let mut serial = SERIAL.borrow_ref_mut(cs);
                            let serial = serial.as_mut().unwrap();
                            for c in buf.iter() {
                                block!(serial.write(*c)).ok();
                            }
                        });
                    }
                },
            }
        }
    }
}

impl Source<SerialEvent> for Serial {
    fn add_handler(&mut self, handler: Box<dyn Handler<SerialEvent>>) {
        self.emitter.borrow_mut().add_handler(handler);
    }
}

// rx_fifo_full_threshold
const READ_BUF_SIZE: usize = 64;
#[interrupt]
fn UART0() {
    critical_section::with(|cs| {
        let mut serial = SERIAL.borrow_ref_mut(cs);
        let serial = serial.as_mut().unwrap();

        let mut buf = Vec::<u8>::new();
        while let nb::Result::Ok(c) = serial.read() {
            buf.push(c);
        }
        RXD_CHANNEL.try_send(SerialEvent::RecvBytes(buf)).ok();

        serial.reset_at_cmd_interrupt();
        serial.reset_rx_fifo_full_interrupt();
    });
}
