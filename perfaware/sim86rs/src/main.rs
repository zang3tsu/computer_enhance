use std::env;
use std::fmt::format;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;

use env_logger::{Builder, Target};
use log::{error, info, trace, warn, Level, LevelFilter};

mod decoders;
mod decoding_table;
mod readers;

use decoding_table::*;

fn main() {
    Builder::new()
        .target(Target::Stdout) // Output all logs to stdout
        .filter_level(LevelFilter::Debug) // Set the minimum log level to Info
        .format(|buf, record| match record.level() {
            Level::Info => {
                writeln!(buf, "{}", record.args())
            }
            _ => {
                writeln!(buf, "; [{}] {}", record.level(), record.args())
            }
        }) // Custom message format
        .init();

    // Get command line arguments
    let args: Vec<String> = env::args().collect();

    // Check if the correct number of arguments are provided
    if args.len() < 2 {
        error!("Usage: {} <file_path>", args[0]);
        std::process::exit(1);
    }

    // Read the file
    let file_path = &args[1];
    let file = match File::open(file_path) {
        Ok(file) => file,
        Err(error) => {
            error!("Error opening file {}: {}", file_path, error);
            std::process::exit(1);
        }
    };
    info!("; {}", file_path);
    info!("BITS 16");

    // Read the file contents into a byte buffer
    let mut buf_reader = BufReader::new(file);
    let mut buffer = Vec::new();
    match buf_reader.read_to_end(&mut buffer) {
        Ok(_) => {}
        Err(error) => {
            error!("Error reading file {}: {}", file_path, error);
            std::process::exit(1);
        }
    };

    let mut iterator = buffer.iter();

    while let Some(byte) = iterator.next() {
        decode_first_byte(*byte, &mut iterator);
    }
}
