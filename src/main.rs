extern crate notify;

use notify::{Watcher, RecursiveMode, RawEvent, raw_watcher};
use notify::op::Op;
use std::sync::mpsc::{channel,Sender};

extern crate app_dirs;
use app_dirs::*;

use std::fs::{File,OpenOptions};
use std::path::Path;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Read;
use std::thread;

#[macro_use] extern crate lazy_static;
extern crate regex;
use regex::Regex;

mod ten_minutes;
use ten_minutes::MeterControlMessage;

fn read_file_length(path: &std::path::Path) -> Result<usize,std::io::Error> {
    match std::fs::metadata(path) {
        Ok(m) => Ok(m.len() as usize),
        Err(e) => Err(e),
    }
}

trait CameraEventWatcher {
    // Data capture started.
    fn on_capture_start(&self, line: &str);

    // Data capture stopped.
    fn on_capture_stop(&self, line: &str);

    // Camera capture started.
    fn on_camera_start(&self, line: &str);

    // Preparing to get Webcamshot.
    fn on_camera_prepare(&self, line: &str);

    // Got picture from the webcam.
    fn on_camera_finish(&self, line: &str);

    // Saved an image to
    fn on_webcam_image_save(&self, line: &str);

    // Saved an image to
    fn on_screenshot_save(&self, line: &str);
}

struct SystrayCameraWatcher {
    tx : Sender<MeterControlMessage>
}
impl CameraEventWatcher for SystrayCameraWatcher {
    fn on_capture_start(&self, line: &str) {
        println!("{}", line);
    }

    fn on_capture_stop(&self, line: &str) {
        println!("{}", line);
    }

    fn on_camera_start(&self, line: &str) {
        println!("{}", line);
    }

    fn on_camera_prepare(&self, line: &str) {
        println!("{}", line);
    }

    fn on_camera_finish(&self, line: &str) {
        println!("{}", line);
        self.tx.send(MeterControlMessage::PhotoDone).ok();
    }

    fn on_webcam_image_save(&self, line: &str) {
        println!("WEBCAM {}", line);
    }

    fn on_screenshot_save(&self, line: &str) {
        println!("SCREENSHOT {}", line);
        self.tx.send(MeterControlMessage::ScreenShotDone).ok();
    }
}

pub struct FileScanner {
    file_name: Box<Path>,
    file: File,
    last_byte_read: usize,
    camera_watcher: Box<CameraEventWatcher>,
}

impl FileScanner {
    fn create(file_name: &std::path::Path, camera_watcher: Box<CameraEventWatcher>) -> Result<FileScanner,String> {
        let file : File = OpenOptions::new().read(true).open(file_name)
            .map_err( |err| err.to_string())?;
        let len = read_file_length(file_name)
            .map_err( |err| err.to_string())?;

        Ok(FileScanner {
            file_name: Box::from(file_name),
            file,
            last_byte_read: len,
            camera_watcher,
        })
    }

    fn handle_event(&mut self, op: Op) -> Result<(), String> {
        lazy_static! {
            static ref SAVE_CAMERA: Regex = Regex::new("Saved an image to .*webcam_").unwrap();
            static ref SAVE_SCREENSHOT: Regex = Regex::new("Saved an image to .*screenshot_").unwrap();
        }

        if op != notify::op::WRITE {
            // TODO support handling of logfile rolling
            return Ok(());
        }

        let len = read_file_length(&self.file_name).map_err( |err| err.to_string())?;
        //println!("{:?} {}", op, len);

        let mut buffer = vec![0; len-self.last_byte_read].into_boxed_slice();
        self.file.seek(SeekFrom::Start(self.last_byte_read as u64))
            .map_err( |err| err.to_string())?;
        self.file.read_exact(&mut buffer).map_err( |err| err.to_string())?;

        let buffer = String::from_utf8_lossy(&buffer);

        for s in buffer.split('\n') {
            if String::from(s).contains("Camera capture started.") {
                self.camera_watcher.on_camera_start(s);
            } else if String::from(s).contains("Preparing to get Webcamshot.") {
                self.camera_watcher.on_camera_prepare(s);
            } else if String::from(s).contains("Got picture from the webcam.") {
                self.camera_watcher.on_camera_finish(s);
            } else if String::from(s).contains("Data capture started.") {
                self.camera_watcher.on_capture_start(s);
            } else if String::from(s).contains("Data capture stopped.") {
                self.camera_watcher.on_capture_stop(s);
            } else if SAVE_CAMERA.is_match(s) {
                self.camera_watcher.on_webcam_image_save(s);
            } else if SAVE_SCREENSHOT.is_match(s) {
                self.camera_watcher.on_screenshot_save(s);
            }
        }

        self.last_byte_read = len;

        Ok(())
    }

}

const APP_INFO: AppInfo = AppInfo{name: "Logs", author: "CrossoverWorkSmart"};

fn create_log_watch_thread(ui_tx: Sender<MeterControlMessage>)/* -> JoinHandle<()>*/ {
    thread::spawn(move || {
        let mut path = get_app_root(AppDataType::UserConfig, &APP_INFO).unwrap();
        path.push("deskapp.log");
        let path = path.as_path();

        // Create a channel to receive the events.
        let (tx, rx) = channel();

        // Create a watcher object, delivering raw events.
        // The notification back-end is selected based on the platform.
        let mut watcher = raw_watcher(tx).unwrap();
        let camera = Box::new(SystrayCameraWatcher { tx: ui_tx });
        let mut scanner = FileScanner::create(path, camera).unwrap();

        // Add a path to be watched. All files and directories at that path and
        // below will be monitored for changes.
        watcher.watch(path, RecursiveMode::NonRecursive).unwrap();
        loop {
            match rx.recv() {
                Ok(RawEvent { path: Some(_path), op: Ok(op), cookie: _cookie }) => scanner.handle_event(op).unwrap(),
                Ok(event) => println!("broken event: {:?}", event),
                Err(e) => println!("watch error: {:?}", e),
            }
        }
    });
}

#[cfg(target_os = "windows")]
fn main() {
    let (ui_tx, ui_rx) = channel();
    create_log_watch_thread(ui_tx);
    ten_minutes::TenMinutesMeter::new(ui_rx).main();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regex() {
        let save_camera: Regex = Regex::new("Saved an image to .*webcam_").unwrap();
        let save_screen: Regex = Regex::new("Saved an image to .*screenshot_").unwrap();
        assert!(save_camera.is_match("[2018-02-07 09:30:17,492][INFO ] Saved an image to C:\\Users\\tcich\\AppData\\Roaming\\CrossoverWorkSmart\\DataCapture\\Data_02_07_18_08_30_00\\webcam_08_30_11.dcw"));
        assert!(!save_screen.is_match("[2018-02-07 09:30:17,492][INFO ] Saved an image to C:\\Users\\tcich\\AppData\\Roaming\\CrossoverWorkSmart\\DataCapture\\Data_02_07_18_08_30_00\\webcam_08_30_11.dcw"));

        assert!(!save_camera.is_match("[2018-02-07 09:30:31,562][INFO ] Saved an image to C:\\Users\\tcich\\AppData\\Roaming\\CrossoverWorkSmart\\DataCapture\\Data_02_07_18_08_30_00\\screenshot_08_30_29.dcs"));
        assert!(save_screen.is_match("[2018-02-07 09:30:31,562][INFO ] Saved an image to C:\\Users\\tcich\\AppData\\Roaming\\CrossoverWorkSmart\\DataCapture\\Data_02_07_18_08_30_00\\screenshot_08_30_29.dcs"));
    }
}