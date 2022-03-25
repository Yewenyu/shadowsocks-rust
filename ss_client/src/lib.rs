use std::{sync::Mutex, thread};

mod Client {
    use std::sync::mpsc::{channel, Receiver, Sender};

    pub(crate) struct Client {
        stopSender: Sender<bool>,
        stopReceiver: Receiver<bool>,
        pub isStart: bool,
    }

    impl Client {
        pub fn new() -> Client {
            let (ts, tr) = channel::<bool>();
            return Client {
                stopSender: ts,
                stopReceiver: tr,
                isStart: false,
            };
        }
        pub fn stop(&self) {
            let _ = self.stopSender.send(true);
        }
        pub fn canStop(&self) -> bool {
            let receiver = self.stopReceiver.try_recv();
            match receiver {
                Ok(v) => v,
                Err(_) => false,
            }
        }
        pub fn update(&mut self) {
            self.isStart = false;
            let (ts, tr) = channel::<bool>();
            self.stopSender = ts;
            self.stopReceiver = tr;
        }
    }
}

use shadowsocks_rust::service::local;

#[macro_use]
extern crate lazy_static;
lazy_static! {
    static ref client: Mutex<Client::Client> = Mutex::new(Client::Client::new());
}

pub fn ss_start(path: String, re_start: bool) {
    local::start(path.as_str(), re_start, |ts| {
        let _ = thread::spawn(move || {
            loop {
                let mut c = client.lock().unwrap();
                c.isStart = true;
                if c.canStop() {
                    c.update();
                    break;
                }
            }
            let _ = ts.send(true);
        });
    });
}
pub fn ss_stop() {
    let c = client.lock().unwrap();
    if c.isStart {
        c.stop();
    }
}
