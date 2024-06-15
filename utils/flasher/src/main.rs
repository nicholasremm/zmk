use std::{env, fs, io};
use std::cmp::Reverse;
use std::fs::DirEntry;
use std::path::Path;
use std::time::UNIX_EPOCH;

use regex::Regex;

struct FlasherArgs {
    firmware_dir: String
}

enum FlasherErrors {
    QuitError,
    FlashFailed
}

struct KbModule<'a> {
    name: String,
    serial_dir: Option<String>,
    src_file: &'a DirEntry
}

const FILE_EXTENSION: &str = "uf2";

fn main() {
    let flasher_args = get_args();
    let mut paths: Vec<_> = fs::read_dir(&flasher_args.firmware_dir)
        .expect("Failed to read firmware dir")
        .map(|r| r.unwrap())
        .filter(|de| de.path().is_file() && de.path().extension().map(|e| e == FILE_EXTENSION).unwrap_or(false))
        .collect::<Vec<DirEntry>>();

    paths.sort_by_key(|e| Reverse(e.metadata().and_then(|m| m.created()).unwrap_or(UNIX_EPOCH)));

    for module in [
        KbModule {
            name: String::from("left"),
            serial_dir: None,
            src_file: find_file(&paths, &module_file_pattern("left")).expect("Missing left module file")
        },
        KbModule {
            name: String::from("right"),
            serial_dir: None,
            src_file: find_file(&paths, &module_file_pattern("right")).expect("Missing right module file")
        }
    ] {
        match interactive_flash_file(&module) {
            Ok(_) => (),
            Err(FlasherErrors::FlashFailed) => {
                println!("Flashing failed");
                break;
            },
            _ => break
        }
    };
}

fn get_args() -> FlasherArgs {
    let args: Vec<String> = env::args().collect();
    match &args[..] {
        [_, firmware_dir] =>
            FlasherArgs {
                firmware_dir: String::from(firmware_dir)
            },
        _ => panic!("usage: {} firmware_dir", args[0])
    }
}

fn module_file_pattern(module_name: &str) -> Regex {
    Regex::new(format!(r".*-{}\.{}$", module_name, FILE_EXTENSION).as_str()).unwrap()
}

fn find_file<'a>(paths: &'a Vec<DirEntry>, file_pattern: &Regex) -> Option<&'a DirEntry> {
    paths
        .iter()
        .find(|p| p
              .path()
              .file_name()
              .map(|file| file.to_str().and_then(|f| file_pattern.find(f)).is_some())
              .unwrap_or(false)
             )
}

fn interactive_flash_file(kb_module: &KbModule) -> Result<(), FlasherErrors> {
    let mut dest_dir = match &kb_module.serial_dir {
        Some(dir) => dir.clone(),
        _ => String::new()
    };
    while dest_dir.trim().is_empty() || !Path::new(&dest_dir).exists() {
        println!("Please specify a destination directory");
        let _ = io::stdin().read_line(&mut dest_dir);
        match dest_dir.as_str().trim() {
            "q" => return Err(FlasherErrors::QuitError),
            _ => ()
        };
    }

    let max_fails = 3;
    let mut fail_count = 0;
    while fail_count < max_fails {
        let file_name = kb_module.src_file.file_name();
        println!("Flashing file {} for {} module", file_name.to_str().unwrap(), &kb_module.name);

        let dest_path = Path::new(&dest_dir).join(file_name);
        match fs::copy(kb_module.src_file.path(), dest_path) {
            Ok(_) => {
                println!("Done!");
                break;
            },
            _ => {
                println!("Failed to copy file");
                fail_count += 1;
            }
        };
    }
    
    match fail_count {
        x if x == max_fails => Err(FlasherErrors::FlashFailed),
        _ => Ok(())
    }
}

