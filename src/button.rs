use core::cell::RefCell;
use core::pin::pin;
use core::pin::Pin;
use core::task::Context;
use core::task::Poll;
use core::task::Waker;

use alloc::boxed::Box;
use alloc::rc::Rc;
use embassy_sync::waitqueue::WakerRegistration;
use embassy_time::with_timeout;
use embedded_hal::can::Error;
use embedded_hal::digital::v2::InputPin;
use esp32_hal::gpio::AnyPin;
use esp32_hal::gpio::Gpio0;
use esp32_hal::gpio::Input;
use esp32_hal::gpio::PullDown;
use esp32_hal::interrupt;
use esp32_hal::macros::interrupt;
use esp32_hal::macros::ram;
use esp32_hal::xtensa_lx;
//use esp32_hal::xtensa_lx::mutex::Mutex;
use futures::Future;
use log::info;

use crate::limero::*;


use embassy_sync::waitqueue::AtomicWaker;
use embassy_time::Duration;
use embassy_time::Timer;
use critical_section::Mutex;


#[derive(Debug, Clone)]
pub enum ButtonEvent {
    Released,
    Pressed,
}


pub struct Button {
    pressed: bool,
    emitter: Rc<RefCell<Emitter<ButtonEvent>>>,
 //   pin: Box<dyn InputPin<Error = ()>>,
}

impl Button {
    pub fn new(pin:AnyPin<Input<PullDown>>) -> Self {
        let reg = WakerRegistration::new();
        
        Button {
            pressed: false,
            emitter: Rc::new(RefCell::new(Emitter::new())),
       //     pin: Box::new(pin),
        }
    }
    pub async fn run(&self) {
        BUTTON.borrow_mut().replace(self.handler());
        BUTTON_PIN.borrow_mut().replace(self.pin);

        critical_section::with(|cs| BUTTON_PIN.borrow_ref_mut(cs).replace(self.pin));
    
    

        Timer::after(Duration::from_millis(u64::MAX)).await;
    }
    pub fn emit(&mut self, event: ButtonEvent) {
        self.emitter.borrow().emit(event);
    }
}

impl  Source<ButtonEvent> for Button {
    fn add_handler(& mut self, handler: Box<dyn Handler<ButtonEvent>>) {
        self.emitter.borrow_mut().add_handler(handler);
    }
}

impl Sink<ButtonEvent> for Button {
    fn handler(&self) -> Box<dyn Handler<ButtonEvent>> {
        struct ButtonHandler {
            emitter: Rc<RefCell<Emitter<ButtonEvent>>>,
        }
        impl<'a> Handler<ButtonEvent> for ButtonHandler {
            fn handle(&self, event: ButtonEvent) {
                info!("ButtonHandler {:?}", event);
                let _ = self.emitter.borrow().emit(event) ;
            }
        }
        Box::new(ButtonHandler {
            emitter: self.emitter.clone(),
        })
    }
}

static BUTTON: Mutex<RefCell<Option<Box<dyn Sink<ButtonEvent>>>>> = Mutex::new(RefCell::new(None));
static BUTTON_PIN : Mutex<RefCell<Option<AnyPin<PullDown>>>> = Mutex::new(RefCell::new(None));
static BUTTON_P: Mutex<RefCell<Option<Gpio0<Input<PullDown>>>>> = Mutex::new(RefCell::new(None));

#[ram]
#[interrupt]
unsafe fn GPIO() {
    esp_println::println!(
        "GPIO Interrupt with priority {}",
        xtensa_lx::interrupt::get_level()
    );

    critical_section::with(|cs| {
        BUTTON
            .borrow_ref_mut(cs)
            .borrow_mut()
            .as_mut()
            .unwrap()
            .clear_interrupt();
    });
}

