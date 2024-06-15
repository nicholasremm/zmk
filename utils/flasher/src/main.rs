use std::cmp::Reverse;
use std::fs::DirEntry;
use std::path::Path;
use std::time::UNIX_EPOCH;
use std::{env, fs, io};

use regex::Regex;

struct FlasherArgs {
    firmware_dir: String,
}

enum FlasherErrors {
    FlashFailed,
}

struct KbModule<'a> {
    name: String,
    src_file: &'a DirEntry,
    default_serial_dir: Option<String>,
}

const FILE_EXTENSION: &str = "uf2";

fn main() {
    let flasher_args = get_args();
    let mut paths: Vec<_> = fs::read_dir(&flasher_args.firmware_dir)
        .expect("Failed to read firmware dir")
        .map(|r| r.unwrap())
        .filter(|de| {
            de.path().is_file()
                && de
                    .path()
                    .extension()
                    .map(|e| e == FILE_EXTENSION)
                    .unwrap_or(false)
        })
        .collect::<Vec<DirEntry>>();

    paths.sort_by_key(|e| Reverse(e.metadata().and_then(|m| m.created()).unwrap_or(UNIX_EPOCH)));

    let mut dest_dir = String::new();
    for module in [
        KbModule {
            name: String::from("left"),
            src_file: find_file(&paths, &module_file_pattern("left"))
                .expect("Missing left module file"),
            default_serial_dir: None,
        },
        KbModule {
            name: String::from("right"),
            src_file: find_file(&paths, &module_file_pattern("right"))
                .expect("Missing right module file"),
            default_serial_dir: None,
        },
    ] {
        println!("Processing {} module", &module.name);
        dest_dir = match &module.default_serial_dir {
            Some(d) => d.to_string(),
            _ => match dest_dir {
                dir if dir.is_empty() => get_dest_dir().expect("Failed to get destination dir"),
                dir => dir
            }
        };
        match interactive_flash_file(&module, &dest_dir) {
            Ok(_) => (),
            _ => {
                println!("Flashing failed");
                break;
            }
        }
    }
}

fn get_args() -> FlasherArgs {
    let args: Vec<String> = env::args().collect();
    match &args[..] {
        [_, firmware_dir] => FlasherArgs {
            firmware_dir: String::from(firmware_dir),
        },
        _ => panic!("usage: {} firmware_dir", args[0]),
    }
}

fn module_file_pattern(module_name: &str) -> Regex {
    Regex::new(format!(r".*-{}\.{}$", module_name, FILE_EXTENSION).as_str()).unwrap()
}

fn find_file<'a>(paths: &'a Vec<DirEntry>, file_pattern: &Regex) -> Option<&'a DirEntry> {
    paths.iter().find(|p| {
        p.path()
            .file_name()
            .map(|file| file.to_str().and_then(|f| file_pattern.find(f)).is_some())
            .unwrap_or(false)
    })
}

fn get_dest_dir() -> Option<String> {
    let mut dest_dir = String::new();
    while dest_dir.trim().is_empty() || !Path::new(&dest_dir).exists() {
        println!("Please specify a valid destination directory");
        let _ = io::stdin().read_line(&mut dest_dir);
        dest_dir = dest_dir.trim().to_string();
        match dest_dir.as_str().trim() {
            "q" => return None,
            _ => ()
        };
    }

    Some(dest_dir)
}

fn interactive_flash_file(kb_module: &KbModule, dest_dir: &String) -> Result<(), FlasherErrors> {
    let file_name = kb_module.src_file.file_name();
    println!(
        "Flashing file {} for {} module",
        file_name.to_str().unwrap(),
        &kb_module.name
    );

    let max_fails = 3;
    let mut fail_count = 0;
    while fail_count < max_fails {
        let dest_path = Path::new(&dest_dir).join(&file_name);
        match fs::copy(kb_module.src_file.path(), dest_path) {
            Ok(_) => {
                println!("Done!");
                break;
            }
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
