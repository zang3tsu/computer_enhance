use env_logger::{Builder, Target};
use lazy_static::lazy_static;
use log::{debug, error, info, trace, warn, LevelFilter};
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;

lazy_static! {
    static ref REG_FIELD_ENCODING: HashMap<u8, HashMap<u8, &'static str>> = {
        let mut map = HashMap::new();
        map.insert(0b000, {
            let mut map = HashMap::new();
            map.insert(0b0, "AL");
            map.insert(0b1, "AX");
            map
        });
        map.insert(0b001, {
            let mut map = HashMap::new();
            map.insert(0b0, "CL");
            map.insert(0b1, "CX");
            map
        });
        map.insert(0b010, {
            let mut map = HashMap::new();
            map.insert(0b0, "DL");
            map.insert(0b1, "DX");
            map
        });
        map.insert(0b011, {
            let mut map = HashMap::new();
            map.insert(0b0, "BL");
            map.insert(0b1, "BX");
            map
        });
        map.insert(0b100, {
            let mut map = HashMap::new();
            map.insert(0b0, "AH");
            map.insert(0b1, "SP");
            map
        });
        map.insert(0b101, {
            let mut map = HashMap::new();
            map.insert(0b0, "CH");
            map.insert(0b1, "BP");
            map
        });
        map.insert(0b110, {
            let mut map = HashMap::new();
            map.insert(0b0, "DH");
            map.insert(0b1, "SI");
            map
        });
        map.insert(0b111, {
            let mut map = HashMap::new();
            map.insert(0b0, "BH");
            map.insert(0b1, "DI");
            map
        });
        map
    };
}

const LOG_LEVEL: LevelFilter = LevelFilter::Debug;

fn decode_move_register_memory_to_from_register(byte: u8, iterator: &mut std::slice::Iter<u8>) {
    debug!("  MOV: Register/memory to/from register");

    const D_MASK: u8 = 0b0000_0010;
    const W_MASK: u8 = 0b0000_0001;

    let d_field = (byte & D_MASK) >> 1;
    let w_field = byte & W_MASK;
    debug!("    D: {:01b}", d_field);
    debug!("    W: {:01b}", w_field);

    let next_byte = iterator.next().unwrap();

    debug!("  Next byte: {:08b}", next_byte);

    const MOD_MASK: u8 = 0b1100_0000;
    const REG_MASK: u8 = 0b0011_1000;
    const RM_MASK: u8 = 0b0000_0111;

    let mod_field = (next_byte & MOD_MASK) >> 6;
    let reg_field = (next_byte & REG_MASK) >> 3;
    let rm_field = next_byte & RM_MASK;

    match mod_field {
        0b11 => {
            debug!("    Mod: {:02b}", mod_field);
            debug!("    Reg: {:03b}", reg_field);
            let reg_field_map = REG_FIELD_ENCODING.get(&reg_field).unwrap();
            let reg_field_encoding = reg_field_map.get(&w_field).unwrap();
            debug!("    Reg encoding: {}", reg_field_encoding);

            debug!("    R/M: {:03b}", rm_field);
            let rm_field_map = REG_FIELD_ENCODING.get(&rm_field).unwrap();
            let rm_field_encoding = rm_field_map.get(&w_field).unwrap();
            debug!("    R/M encoding: {}", rm_field_encoding);
            info!("MOV {}, {}", rm_field_encoding, reg_field_encoding);
        }
        _ => {
            debug!("    Mod: {}", mod_field);
        }
    }
}

fn main() {
    Builder::new()
        .filter_level(LOG_LEVEL) // Set the minimum log level to Info
        .target(Target::Stdout) // Output all logs to stdout
        .format(|buf, record| writeln!(buf, "{}", record.args())) // Custom message format
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
    info!(";{}", file_path);
    info!("BITS 16");

    // Read the file contents into a byte buffer
    let mut buf_reader = BufReader::new(file);
    let mut buffer = Vec::new();
    match buf_reader.read_to_end(&mut buffer) {
        Ok(_) => {
            debug!("File contents (bytes): {:?}", buffer);
        }
        Err(error) => {
            error!("Error reading file {}: {}", file_path, error);
            std::process::exit(1);
        }
    };

    let mut iterator = buffer.iter();

    const MOV_REG_MEM_TO_FRO_REG_MASK: u8 = 0b1111_1100;
    const MOV_REG_MEM_TO_FRO_REG_OPCODE: u8 = 0b1000_1000;
    const MOV_IMM_TO_REG_MASK: u8 = 0b1111_0000;
    const MOV_IMM_TO_REG_OPCODE: u8 = 0b1011_0000;

    while let Some(byte) = iterator.next() {
        debug!("Byte: {:08b}", byte);
        match byte {
            byte if (byte & MOV_REG_MEM_TO_FRO_REG_MASK) == MOV_REG_MEM_TO_FRO_REG_OPCODE => {
                decode_move_register_memory_to_from_register(*byte, &mut iterator)
            }
            _ => {}
        }
    }
}
