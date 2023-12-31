#![no_std]
#![no_main]
#![allow(unused_imports)]

mod limero;
mod led;
use embassy_executor::main;
use led::*;
extern crate alloc;
use core::mem::MaybeUninit;
use esp32_hal::{clock::ClockControl, peripherals::Peripherals, prelude::*, Delay};
use esp_backtrace as _;
use esp_hal_common::IO;
use esp_println::println;

use esp_wifi::{initialize, EspWifiInitFor};

use esp32_hal::{timer::TimerGroup, Rng};
#[global_allocator]
static ALLOCATOR: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

fn init_heap() {
    const HEAP_SIZE: usize = 32 * 1024;
    static mut HEAP: MaybeUninit<[u8; HEAP_SIZE]> = MaybeUninit::uninit();

    unsafe {
        ALLOCATOR.init(HEAP.as_mut_ptr() as *mut u8, HEAP_SIZE);
    }
}



/*#[embassy_executor::main]
async fn main(spawner: Spawner) {*/
#[entry]
fn main() -> ! {
    init_heap();
    let mut executor = Executor::new(); 
    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();
    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
    let mut led = io.pins.gpio2.into_push_pull_output();
    led.set_high().unwrap();

    let clocks = ClockControl::max(system.clock_control).freeze();
    let mut delay = Delay::new(&clocks);

    // setup logger
    // To change the log_level change the env section in .cargo/config.toml
    // or remove it and set ESP_LOGLEVEL manually before running cargo run
    // this requires a clean rebuild because of https://github.com/rust-lang/cargo/issues/10358
    esp_println::logger::init_logger_from_env();
    log::info!("Logger is setup");
    println!("Hello world!");
    let timer = TimerGroup::new(peripherals.TIMG1, &clocks).timer0;
    let _init = initialize(
        EspWifiInitFor::Wifi,
        timer,
        Rng::new(peripherals.RNG),
        system.radio_clock_control,
        &clocks,
    )
    .unwrap();
    loop {
        println!("Loop...");
        delay.delay_ms(500u32);
    }
}
