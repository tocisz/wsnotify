extern crate systray;

use std::sync::mpsc::Receiver;

use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::process;
use std::fmt;

pub enum MeterControlMessage {
    PhotoDone, ScreenShotDone
}

#[derive(PartialEq)]
#[derive(Debug)]
enum Icon {
    Stop, OK, Smile, Warning
}

impl fmt::Display for Icon {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

pub struct TenMinutesMeter {
    app : systray::Application,
    rx : Receiver<MeterControlMessage>,
    last_reminder : u16,
    photo_done : bool,
    screenshot_done : bool,
    current_icon : Icon,
}

const TEN_MINUTES : u16 = 600;

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
                    photo_done: false,
                    screenshot_done: false,
                    current_icon: Icon::Stop,
                }
            },
            Err(_) => panic!("Can't create window!")
        }
    }

    fn init(app : &mut systray::Application) -> () {
        app.set_icon_from_resource(&"Stop".to_string()).ok();
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
                    MeterControlMessage::PhotoDone => self.photo_done = true,
                    MeterControlMessage::ScreenShotDone => self.screenshot_done = true,
                }
            }

            let since_epoch = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
            let reminder = (since_epoch.as_secs() % (TEN_MINUTES as u64)) as u16;

            if self.last_reminder > reminder {
                self.photo_done = false;
                self.screenshot_done = false;
            }

            let icon : Icon;
            if TEN_MINUTES - reminder <= 5 { // Warn 5s before end of timecard
                icon = Icon::Warning;
            } else if self.photo_done && self.screenshot_done {
                icon = Icon::Smile;
            } else if self.photo_done {
                icon = Icon::OK;
            } else {
                icon = Icon::Stop;
            }

            if icon != self.current_icon {
                self.set_icon(icon);
            }

            self.last_reminder = reminder;
        }
    }

    fn set_icon(&mut self, icon : Icon) {
        self.app.set_icon_from_resource(&icon.to_string()).ok();
        self.current_icon = icon;
    }
}

