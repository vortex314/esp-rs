use core::{cell::RefCell, fmt::Write, panic::PanicInfo};
use critical_section::Mutex;
use embassy_futures::yield_now;
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, signal::Signal};
use esp32_hal::{
    interrupt,
    peripherals::{self, UART0},
    prelude::*,
    timer::Timer,
    uart::{config::AtCmdConfig, UartRx, UartTx},
    Uart, IO,
};
use nb::block;

pub enum SerialCmd<'a> {
    SendBytes(&'a [u8]),
}

pub enum SerialEvent<'a> {
    RecvBytes(&'a [u8]),
}

pub struct Serial {
    // tx: UartTx<'a, UART0>,
    // rx: UartRx<'a, UART0>,
    tx_buf: [u8; 256],
    tx_buf_len: usize,
    rx_buf: [u8; 256],
    rx_buf_len: usize,
}

static SERIAL: Mutex<RefCell<Option<Uart<UART0>>>> = Mutex::new(RefCell::new(None));

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
            //    tx,
            //    rx,
            tx_buf: [0; 256],
            tx_buf_len: 0,
            rx_buf: [0; 256],
            rx_buf_len: 0,
        }
    }

    pub async fn run(&mut self) {
        loop {
            critical_section::with(|cs| {
                let mut serial = SERIAL.borrow_ref_mut(cs);
                let serial = serial.as_mut().unwrap();
                //   writeln!(serial, "Hello World! Send a single `#` character or send at least 30 characters and see the interrupts trigger.").ok();
            });
            yield_now().await;
        }
    }
}

use static_cell::make_static;

// rx_fifo_full_threshold
const READ_BUF_SIZE: usize = 64;
#[interrupt]
fn UART0() {
    critical_section::with(|cs| {
        let mut serial = SERIAL.borrow_ref_mut(cs);
        let serial = serial.as_mut().unwrap();

        let mut cnt = 0;
        while let nb::Result::Ok(_c) = serial.read() {
            cnt += 1;
        }
        writeln!(serial, "Read {} bytes", cnt,).ok();

        writeln!(
            serial,
            "Interrupt AT-CMD: {} RX-FIFO-FULL: {}",
            serial.at_cmd_interrupt_set(),
            serial.rx_fifo_full_interrupt_set(),
        )
        .ok();

        serial.reset_at_cmd_interrupt();
        serial.reset_rx_fifo_full_interrupt();
    });
}
