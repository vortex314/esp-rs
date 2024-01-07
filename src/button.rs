// use core::borrow::BorrowMut;
use core::cell::RefCell;
use core::mem::MaybeUninit;
use core::pin::pin;
use core::task::Context;
use core::task::Poll;
use core::task::Waker;

use alloc::boxed::Box;
use alloc::rc::Rc;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::waitqueue::WakerRegistration;
use embassy_time::with_timeout;
use embedded_hal::can::Error;
use embedded_hal::digital::v2::InputPin;
use esp32_hal::gpio::AnyPin;
use esp32_hal::gpio::Event;
use esp32_hal::gpio::Gpio0;
use esp32_hal::gpio::GpioPin;
use esp32_hal::gpio::Input;
use esp32_hal::gpio::Pin;
use esp32_hal::gpio::PullDown;
use esp32_hal::interrupt;
use esp32_hal::macros::interrupt;
use esp32_hal::macros::ram;
use esp32_hal::peripherals;
use esp32_hal::xtensa_lx;
//use esp32_hal::xtensa_lx::mutex::Mutex;
use futures::Future;
use log::info;

use crate::limero::*;
use critical_section::Mutex;
use embassy_sync::waitqueue::AtomicWaker;
use embassy_time::Duration;
use embassy_time::Timer;
use static_cell::*;

#[derive(Debug, Clone)]
pub enum ButtonEvent {
    Released,
    Pressed,
}

static BUTTON_PIN: Mutex<RefCell<Option<Gpio0<Input<PullDown>>>>> = Mutex::new(RefCell::new(None));
static CHANNEL : Channel<CriticalSectionRawMutex, ButtonEvent, 3> = Channel::new();

pub struct Button {
    emitter: Rc<RefCell<Emitter<ButtonEvent>>>,
    channel : &'static Channel<CriticalSectionRawMutex, ButtonEvent, 3>,
}

impl Button {
    pub fn new(mut pin: GpioPin<Input<PullDown>, 0>) -> Self {
        pin.listen(Event::AnyEdge);

        critical_section::with(|cs| {
            BUTTON_PIN.borrow_ref_mut(cs).replace(pin);
        });
        Button {
            emitter: Rc::new(RefCell::new(Emitter::new())),
            channel : &CHANNEL,
        }
    }
    pub async fn run(&self) {
        info!("Button run");
        interrupt::enable(peripherals::Interrupt::GPIO, interrupt::Priority::Priority2).unwrap();

        loop {
            let event = self.channel.receive().await;
            info!("Button event {:?}", event);
            self.emitter.borrow().emit(event);
        }
    }
}

impl Source<ButtonEvent> for Button {
    fn add_handler(&self, handler: Box<dyn Handler<ButtonEvent>>) {
        self.emitter.borrow_mut().add_handler(handler);
    }
}


#[ram]
#[interrupt]
unsafe fn GPIO() {
    critical_section::with(|cs| {
        if BUTTON_PIN
            .borrow_ref_mut(cs)
            .as_mut()
            .unwrap()
            .is_low()
            .unwrap()
        {
            let _ = CHANNEL.try_send(ButtonEvent::Pressed);
        } else {
            let _ = CHANNEL.try_send(ButtonEvent::Released);
        };
        BUTTON_PIN
            .borrow_ref_mut(cs)
            .as_mut()
            .unwrap()
            .clear_interrupt();
    });
}
