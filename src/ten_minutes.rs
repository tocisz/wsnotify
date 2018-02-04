extern crate systray;

use std::sync::mpsc::Receiver;

use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::process;

pub enum MeterControlMessage {
    MarkSafe
}

pub struct TenMinutesMeter {
    app : systray::Application,
    rx : Receiver<MeterControlMessage>,
    last_reminder : u16,
}

impl TenMinutesMeter {
    pub fn new(rx : Receiver<MeterControlMessage>) -> TenMinutesMeter {
        match systray::Application::new() {
            Ok(w) => {
                let mut app = w;
                TenMinutesMeter::init(&mut app);
                TenMinutesMeter {
                    app,
                    rx,
                    last_reminder: 0,
                }
            },
            Err(_) => panic!("Can't create window!")
        }
    }

    fn init(app : &mut systray::Application) -> () {
        app.set_icon_from_resource(&"Stop".to_string()).ok();
        //    app.add_menu_item(&"Print a thing".to_string(), |_| {
        //        println!("Printing a thing!");
        //    }).ok();
        //    app.add_menu_separator().ok();
        app.add_menu_item(&"Quit".to_string(), |window| {
            window.shutdown().ok();
            window.quit();
            process::exit(0);
        }).ok();
    }

    pub fn main(&mut self) -> () {
        loop {
            match self.app.wait_for_message_timeout(Duration::from_secs(1)) {
                Ok(()) => (),
                Err(e) => {
                    println!("{:?}", e);
                    return;
                },
            }

            for m in self.rx.try_iter() {
                match m {
                    MeterControlMessage::MarkSafe => self.mark_safe()
                }
            }

            let since_epoch = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
            let reminder = (since_epoch.as_secs() % 600) as u16;

            if self.last_reminder > reminder {
                self.mark_unsafe();
            }

            self.last_reminder = reminder;
        }
    }

    pub fn mark_unsafe(&self) {
        self.app.set_icon_from_resource(&"Stop".to_string()).ok();
    }

    pub fn mark_safe(&self) {
        self.app.set_icon_from_resource(&"OK".to_string()).ok();
    }
}

