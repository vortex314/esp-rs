#![no_std]
#![no_main]
#![allow(unused_imports)]
#![feature(type_alias_impl_trait)]

mod limero;
mod led;
use embassy_futures::select;
use led::*;
extern crate alloc;
use core::mem::MaybeUninit;
use esp_backtrace as _;
use esp_println::println;


use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp32_hal::{
    clock::ClockControl,
    embassy::{self},
    peripherals::Peripherals,
    prelude::*,
    timer::TimerGroup,
    Delay,
    IO,
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
    led_pin.set_low().unwrap();
    let mut led_task = Led::new(led_pin.degrade(), 3);
    led_task.handler().handle(LedCmd::Blink(100));
    led_task.run().await;

    let mut delay = Delay::new(&clocks);

    loop {
        println!("Loop...");
        delay.delay_ms(500u32);
    }
}
