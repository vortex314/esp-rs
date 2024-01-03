use core::{fmt::Write, panic::PanicInfo};

use esp32_hal::{
    clock_control::{sleep, ClockControl, XTAL_FREQUENCY_AUTO},
    dport::Split,
    dprintln,
    prelude::*,
    serial::{config::Config, Pins, Serial},
    target,
    timer::Timer,
    Uart,
};
enum SerialCmd {
    SendBytes([u8]),
}

enum SerialEvent {
    RecvBytes([u8]),
}

struct Serial {
      tx: peripherals::UART0,
        rx: peripherals::UART0,
        tx_pin: IO<peripherals::UART0_TX>,
        rx_pin: IO<peripherals::UART0_RX>,
        tx_buf: [u8; 256],
        tx_buf_len: usize,
        rx_buf: [u8; 256],
        rx_buf_len: usize,   
}

impl Serial {
    pub fn new(tx: peripherals::UART0, rx: peripherals::UART0, tx_pin: IO<peripherals::UART0_TX>, rx_pin: IO<peripherals::UART0_RX>) -> Self {
        Self {
            tx,
            rx,
            tx_pin,
            rx_pin,
            tx_buf: [0; 256],
            tx_buf_len: 0,
            rx_buf: [0; 256],
            rx_buf_len: 0,
        }
    }

    pub fn send_bytes(&mut self, bytes: [u8]) {
        self.tx_buf = bytes;
        self.tx_buf_len = bytes.len();
        self.tx.ier.modify(|_, w| w.txwm().txwm_0());
    }

    pub fn recv_bytes(&mut self) -> [u8] {
        self.rx_buf
    }

    async fn run(&mut self) {
    let signal = &*make_static!(Signal::new());
    spawner.spawn(reader(self.rx, &signal)).ok();
    spawner.spawn(writer(self.tx, &signal)).ok();
    }
}



use static_cell::make_static;

// rx_fifo_full_threshold
const READ_BUF_SIZE: usize = 64;
// EOT (CTRL-D)
const AT_CMD: u8 = 0x04;

#[embassy_executor::task]
async fn writer(mut tx: UartTx<'static, UART0>, signal: &'static Signal<NoopRawMutex, usize>) {
    use core::fmt::Write;
    embedded_io_async::Write::write(
        &mut tx,
        b"Hello async serial. Enter something ended with EOT (CTRL-D).\r\n",
    )
    .await
    .unwrap();
    embedded_io_async::Write::flush(&mut tx).await.unwrap();
    loop {
        let bytes_read = signal.wait().await;
        signal.reset();
        write!(&mut tx, "\r\n-- received {} bytes --\r\n", bytes_read).unwrap();
        embedded_io_async::Write::flush(&mut tx).await.unwrap();
    }
}

#[embassy_executor::task]
async fn reader(mut rx: UartRx<'static, UART0>, signal: &'static Signal<NoopRawMutex, usize>) {
    const MAX_BUFFER_SIZE: usize = 10 * READ_BUF_SIZE + 16;

    let mut rbuf: [u8; MAX_BUFFER_SIZE] = [0u8; MAX_BUFFER_SIZE];
    let mut offset = 0;
    loop {
        let r = embedded_io_async::Read::read(&mut rx, &mut rbuf[offset..]).await;
        match r {
            Ok(len) => {
                offset += len;
                esp_println::println!("Read: {len}, data: {:?}", &rbuf[..offset]);
                offset = 0;
                signal.signal(len);
            }
            Err(e) => esp_println::println!("RX Error: {:?}", e),
        }
    }
}
