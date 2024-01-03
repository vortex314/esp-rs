#![no_std]
#![no_main]
#![allow(unused_imports)]
#![feature(type_alias_impl_trait)]

mod limero;
mod led;
use led::*;
mod button;
use button::*;
mod serial;
use serial::*;
extern crate alloc;
use core::mem::MaybeUninit;
use esp_backtrace as _;
use esp_println::println;

use embassy_futures::select;

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp32_hal::{
    clock::ClockControl,
    embassy::{self},
    peripherals::{self,Peripherals},
    prelude::*,
    timer::TimerGroup,
    Delay,
    IO, gpio::Event, interrupt,
    uart::{config::AtCmdConfig, UartRx, UartTx},
    Uart,
};

use crate::limero::Sink;

#[global_allocator]
static ALLOCATOR: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

fn init_heap() {
    const HEAP_SIZE: usize = 32 * 1024;
    static mut HEAP: MaybeUninit<[u8; HEAP_SIZE]> = MaybeUninit::uninit();

    unsafe {
        ALLOCATOR.init(HEAP.as_mut_ptr() as *mut u8, HEAP_SIZE);
    }
}

#[embassy_executor::task]
async fn writer() {

}

#[main]
async fn main(spawner: Spawner) {
    init_heap();
    esp_println::logger::init_logger_from_env();
    println!("main started");
    log::info!("Logger is setup");


    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::max(system.clock_control).freeze();
    let timer_group0 = TimerGroup::new(peripherals.TIMG0, &clocks);
    embassy::init(&clocks, timer_group0.timer0);

    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
    let mut led_pin = io.pins.gpio2.into_push_pull_output();
    let mut button_pin = io.pins.gpio0.into_pull_down_input();   
    let mut button_task = Button::new(button_pin);

    let mut led_task = Led::new(led_pin.degrade(), 3);
    led_task.handler().handle(LedCmd::Blink(100));
    led_task.run().await;

    let mut uart0 = Uart::new(peripherals.UART0, &clocks);
    uart0.set_at_cmd(AtCmdConfig::new(None, None, None, AT_CMD, None));
    uart0
        .set_rx_fifo_full_threshold(READ_BUF_SIZE as u16)
        .unwrap();
    let (tx, rx) = uart0.split();
    let mut serial = Serial::new(tx, rx, io.pins.gpio1, io.pins.gpio3);

    let mut delay = Delay::new(&clocks);

    loop {
        println!("Loop...");
        delay.delay_ms(500u32);
    }
}
