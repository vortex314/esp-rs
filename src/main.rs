#![no_std]
#![no_main]
#![allow(unused_imports)]
#![feature(type_alias_impl_trait)]
#![allow(unused_mut)]
mod limero;
use alloc::vec;
use limero::*;
mod led;
use led::*;
mod button;
use button::*;
mod serial;
use log::info;
use serial::*;
extern crate alloc;
use core::mem::MaybeUninit;
use esp_backtrace as _;
use esp_println::println;

use embassy_futures::select::{self, select3};

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp32_hal::{
    clock::ClockControl,
    embassy::{self},
    gpio::Event,
    interrupt,
    peripherals::{self, Peripherals},
    prelude::*,
    timer::TimerGroup,
    uart::{config::AtCmdConfig, UartRx, UartTx},
    Delay, Uart, IO,
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
async fn writer() {}

#[main]
async fn main(_spawner: Spawner) {
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
    let led_pin = io.pins.gpio2.into_push_pull_output();
    let button_pin = io.pins.gpio0.into_pull_down_input();
    let mut button_task = Button::new(button_pin);

    let mut led_task = Led::new(led_pin.degrade(), 3);
    led_task.handler().handle(LedCmd::Blink(1000));

    let uart0 = Uart::new(peripherals.UART0, &clocks);
    let mut serial_task = Serial::new(uart0);

    let mut pressed_led_on = Mapper::new(move |x| match x {
        ButtonEvent::Pressed => LedCmd::Blink(100),
        ButtonEvent::Released => LedCmd::Blink(500),
    });

    let mut serial_input = Mapper::new(move |x| match x {
        SerialEvent::RecvBytes(x) => { println!("SerialEvent::RecvBytes {:?}", x);  },
    });

    let mut serial_output = Mapper::new(move |x| match x {
        ButtonEvent::Pressed => SerialCmd::SendBytes(vec![0x41, 0x42, 0x43]),
        _ => SerialCmd::SendBytes(vec![0x44, 0x45, 0x46]),
    });

    button_task.as_source() >> &pressed_led_on ;// >> &led_task as &dyn Sink<LedCmd>;
    button_task.as_source() >> &serial_output;// >> serial_task;
    serial_task.as_source() >> &serial_input;

    &serial_output.add_sink(serial_task);

    button_task.add_handler(serial_output.handler());
    serial_output.add_handler(serial_task.handler());

    button_task.add_handler(pressed_led_on.handler());
    pressed_led_on.add_handler(led_task.handler());

    serial_task.add_handler(serial_input.handler());

    select3(led_task.run(), button_task.run(), serial_task.run()).await;

    let mut delay = Delay::new(&clocks);

    loop {
        println!("Loop...");
        delay.delay_ms(500u32);
    }
}
