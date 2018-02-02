extern crate notify;

use notify::{Watcher, RecursiveMode, RawEvent, raw_watcher};
use notify::op::Op;
use std::sync::mpsc::channel;

extern crate app_dirs;
use app_dirs::*;

use std::fs::{File,OpenOptions};
use std::path::Path;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Read;
//use std::io::Error;

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
    fn on_image_save(&self, line: &str);
}

struct EchoCameraWatcher {}
impl CameraEventWatcher for EchoCameraWatcher {
    fn on_camera_start(&self, line: &str) {
        println!("{}", line);
    }
    fn on_camera_prepare(&self, line: &str) {
        println!("{}", line);
    }
    fn on_camera_finish(&self, line: &str) {
        println!("{}", line);
    }
    fn on_capture_start(&self, line: &str) {
        println!("{}", line);
    }

    fn on_capture_stop(&self, line: &str) {
        println!("{}", line);
    }

    fn on_image_save(&self, line: &str) {
        println!("{}", line);
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

        if op != notify::op::WRITE {
            // TODO support handling logfile rolling
            return Ok(());
        }

        let len = read_file_length(&self.file_name).map_err( |err| err.to_string())?;
        // println!("{:?} {}", op, len);

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
            } else if String::from(s).contains("Saved an image to ") {
                self.camera_watcher.on_image_save(s);
            }
        }

        self.last_byte_read = len;

        Ok(())
    }

}

const APP_INFO: AppInfo = AppInfo{name: "Logs", author: "CrossoverWorkSmart"};

fn main() {
    let mut path = get_app_root(AppDataType::UserConfig, &APP_INFO).unwrap();
    path.push("deskapp.log");
    let path = path.as_path();

    // Create a channel to receive the events.
    let (tx, rx) = channel();

    // Create a watcher object, delivering raw events.
    // The notification back-end is selected based on the platform.
    let mut watcher = raw_watcher(tx).unwrap();
    let camera = Box::new(EchoCameraWatcher{});
    let mut scanner = FileScanner::create(path, camera).unwrap();

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch(path, RecursiveMode::NonRecursive).unwrap();
    loop {
        match rx.recv() {
            Ok(RawEvent{path: Some(_path), op: Ok(op), cookie: _cookie}) => scanner.handle_event(op).unwrap(),
            Ok(event) => println!("broken event: {:?}", event),
            Err(e) => println!("watch error: {:?}", e),
        }
    }
}