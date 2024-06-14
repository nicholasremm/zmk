use std::{env, fs, io};
use std::cmp::Reverse;
use std::fs::DirEntry;
use std::path::Path;
use std::time::UNIX_EPOCH;

use regex::Regex;

struct FlasherArgs {
    firmware_dir: String
}

struct KbModule<'a> {
    name: String,
    serial_dir: String,
    src_file: &'a DirEntry
}

fn main() {
    let flasher_args = get_args();
    let mut paths: Vec<_> = fs::read_dir(&flasher_args.firmware_dir)
        .expect("Failed to read firmware dir")
        .map(|r| r.unwrap())
        .filter(|de| de.path().is_file() && de.path().extension().map(|e| e == "uf2").unwrap_or(false))
        .collect::<Vec<DirEntry>>();

    paths.sort_by_key(|e| Reverse(e.metadata().and_then(|m| m.created()).unwrap_or(UNIX_EPOCH)));

    for module in [
        KbModule {
            name: String::from("left"),
            serial_dir: String::from("/dev/serial/by-id/some-path-to-left"),
            src_file: find_file(&paths, &module_file_pattern("left")).expect("Missing left module file")
        },
        KbModule {
            name: String::from("right"),
            serial_dir: String::from("/dev/serial/by-id/some-path-to-right"),
            src_file: find_file(&paths, &module_file_pattern("right")).expect("Missing right module file")
        }
    ] {
        if !interactive_flash_file(&module) {
            break;
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
    Regex::new(format!(r".*-{}-.*", module_name).as_str()).unwrap()
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

fn interactive_flash_file(kb_module: &KbModule) -> bool {
    let mut input = String::new();
    let mut success: bool = false;
    while !success {
        println!("\nInsert {} module", kb_module.name);
        let _ = io::stdin().read_line(&mut input);
        match input.as_str().trim() {
            "q"  => break,
            _ => ()
        };

        if !Path::new(&kb_module.serial_dir).exists() {
            println!("Serial directory does not exist");
            continue;
        }

        let file_name = kb_module.src_file.file_name();
        println!("Flashing file {}", file_name.to_str().unwrap());

        let dest_path = Path::new(&kb_module.serial_dir).join(file_name);
        match fs::copy(kb_module.src_file.path(), dest_path) {
            Ok(_) => {
                println!("Done!");
                success = true;
            },
            _ => println!("Failed to copy file")
        };
    }

    success
}

