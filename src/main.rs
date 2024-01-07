#![no_std]
#![no_main]
#![allow(unused_imports)]
#![feature(type_alias_impl_trait)]
#![allow(unused_mut)]
/*
["log",msec,"I",file, line, message   ] => log message
["pub","topic",message] => message can be any json object
["sub","topic"] => subscribe to topic
["uns","topic"] => unsubscribe from topic
["req","topic",message] => request message
["rep","topic",message] => response message

*/
mod limero;
use alloc::vec;
use limero::*;
mod led;
use led::*;
mod button;
use button::*;
mod serial;
use serial::*;
mod pubsub;
use pubsub::*;

extern crate alloc;
use core::mem::MaybeUninit;
use esp_backtrace as _;
use esp_println::println;

use embassy_futures::select::{self, select3, select4};
use log::info;

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
    limero::logger::init_logger(log::LevelFilter::Trace);
    //    esp_println::logger::init_logger_from_env();
    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::max(system.clock_control).freeze();
    let timer_group0 = TimerGroup::new(peripherals.TIMG0, &clocks);
    embassy::init(&clocks, timer_group0.timer0);

    log::info!("Logger is setup");

    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
    let led_pin = io.pins.gpio2.into_push_pull_output();
    let button_pin = io.pins.gpio0.into_pull_down_input();
    let mut button_on_board = Button::new(button_pin);
    let mut pubsub = PubSub::new();

    let mut led_on_board = Led::new(led_pin.degrade(), 3);
    led_on_board.handler().handle(LedCmd::Blink(1000));

    let uart0 = Uart::new(peripherals.UART0, &clocks);
    let mut serial_uart0 = Serial::new(uart0);

    let mut pressed_led_blink_fast = Mapper::new(move |x| match x {
        ButtonEvent::Pressed => LedCmd::Blink(100),
        ButtonEvent::Released => LedCmd::Blink(500),
    });

    let mut serial_input = Mapper::new(move |x| match x {
        SerialEvent::RecvBytes(x) => {
            println!("SerialEvent::RecvBytes {:?}", x);
        }
    });

    let mut button_to_serial = Mapper::new(move |x| match x {
        ButtonEvent::Pressed => SerialCmd::SendBytes("pressed\r\n".as_bytes().to_vec()),
        _ => SerialCmd::SendBytes("released\r\n".as_bytes().to_vec()),
    });

    // let flow1 =  button >> mapper >> led ;
    // select( flow1, flow2 ).await;
    /*source(button_on_board)
    .map(|x| {
        if x == ButtonEvent::Pressed {
            LedCmd::Blink(100)
        } else {
            LedCmd::Blink(500)
        }
    })
    .sink(led_on_board);*/

    source(&button_on_board) >> &pressed_led_blink_fast >> &led_on_board;
    source(&button_on_board) >> &button_to_serial >> &serial_uart0;
    let _ = source(&serial_uart0) >> &serial_input;

    select4(
        led_on_board.run(),
        button_on_board.run(),
        serial_uart0.run(),
        pubsub.run(),
    )
    .await;

    let mut delay = Delay::new(&clocks);

    loop {
        println!("Loop...");
        delay.delay_ms(500u32);
    }
}
