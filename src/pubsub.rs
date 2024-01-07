use crate::limero::{leak_static, Emitter, Handler, Sink, Source};
use alloc::boxed::Box;
use alloc::string::String;
use alloc::{rc::Rc, vec::Vec};
use core::any::Any;
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
use embassy_time::Instant;
use log::info;
use serde::ser::{self, SerializeSeq};
use serde::{Deserialize, Serialize, Serializer};
use serde_json_core::ser::Error;
use serde_json_core::{de::from_slice, ser::to_vec};
//use serde_json_core::ser::Serializer;
use core::str;

trait Subscriber<T> {}
#[derive(Debug, Clone)]
enum PubSubCmd {
    Subscribe(String),
    Unsubscribe(String),
    Publish { topic:String, payload: &'static dyn Any },
    Rxd{ payload:String}
}

enum PubSubEvent {
    Publish { topic: String, payload: &'static dyn Any },
    Txd{ payload:String}
}

pub struct PubSub {
    channel: &'static Channel<NoopRawMutex, PubSubCmd, 3>,
    emitter: Rc<RefCell<Emitter<PubSubEvent>>>,
    buffer: [u8; 128],
}

impl PubSub {
    pub fn new() -> Self {
        let channel = leak_static(Channel::new());
        PubSub {
            channel,
            emitter: Rc::new(RefCell::new(Emitter::new())),
            buffer: [0u8; 128],
        }
    }

    pub fn serialize_publish<T>(&mut self, topic: &str, payload: &T) -> Result<usize, Error>
    where
        T: ser::Serialize,
    {
        let mut serializer = serde_json_core::ser::Serializer::new(&mut self.buffer[..]);
        let mut seq = serializer.serialize_seq(Some(3)).unwrap();
        seq.serialize_element("pub")?;
        seq.serialize_element(topic)?;
        seq.serialize_element(payload)?;
        seq.end()?;
        Ok(serializer.end())
    }

    pub async fn run(&mut self) {
        #[derive(Serialize, Deserialize, Debug,Clone)]
        struct X<'a> {
            lat: f64,
            lon: f64,
            city: &'a str,
            sea: bool,
        }
        let x = X {
            lat: 43.5,
            lon: 12.5,
            city: "Rimini",
            sea: true,
        };
        let msg = PubSubCmd::Publish { topic: String::from("test"), payload:  &x.clone() };

        let cnt = self.serialize_publish("src/esp32/sys/time", &x).unwrap();
        info!(
            " pubsub serialized : {}",
            str::from_utf8(&self.buffer[..cnt]).unwrap()
        );

        loop {
            let cmd = self.channel.receive().await;
            match cmd {
                PubSubCmd::Subscribe(topic) => {
                    info!("subscribe {}", topic);
                }
                PubSubCmd::Unsubscribe(topic) => {
                    info!("unsubscribe {}", topic);
                }
                PubSubCmd::Publish { topic, payload } => {
                    info!("publish {} {:?}", topic, payload);
                }
            }
        }
    }
}

impl Sink<PubSubCmd> for PubSub {
    fn handler(&self) -> Box<dyn Handler<PubSubCmd>> {
        struct PubSubHandler<'a> {
            channel: &'a Channel<NoopRawMutex, PubSubCmd, 3>,
        }
        impl<'a> Handler<PubSubCmd> for PubSubHandler<'a> {
            fn handle(&self, cmd: PubSubCmd) {
                self.channel.try_send(cmd.clone()).expect("send failed");
            }
        }
        Box::new(PubSubHandler {
            channel: self.channel,
        })
    }
}

impl Source<PubSubEvent> for PubSub {
    fn add_handler(&self, handler: Box<dyn Handler<PubSubEvent>>) {
        self.emitter.borrow_mut().add_handler(handler);
    }
}
