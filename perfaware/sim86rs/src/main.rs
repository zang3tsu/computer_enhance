use std::collections::HashMap;
use std::env;
use std::fmt::format;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;

use env_logger::{Builder, Target};
use lazy_static::lazy_static;
use log::{debug, error, info, trace, warn, Level, LevelFilter};

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
    static ref RM_FIELD_ENCODING: HashMap<u8, &'static str> = {
        let mut map = HashMap::new();
        map.insert(0b000, "BX + SI");
        map.insert(0b001, "BX + DI");
        map.insert(0b010, "BP + SI");
        map.insert(0b011, "BP + DI");
        map.insert(0b100, "SI");
        map.insert(0b101, "DI");
        map.insert(0b110, "BP");
        map.insert(0b111, "BX");
        map
    };
}

const MASK_FOUR_BIT: u8 = 0b1111_0000;
const MASK_SEVEN_BIT: u8 = 0b1111_1110;
const MASK_SIX_BIT: u8 = 0b1111_1100;

const MOV_ACC_TO_MEM_OPCODE: u8 = 0b1010_0010;
const MOV_IMM_TO_REG_MEM_OPCODE: u8 = 0b1100_0110;
const MOV_IMM_TO_REG_OPCODE: u8 = 0b1011_0000;
const MOV_MEM_TO_ACC_OPCODE: u8 = 0b1010_0000;
const MOV_REG_MEM_TO_FRO_REG_OPCODE: u8 = 0b1000_1000;

fn read_next_byte_and_combine(word: u16, iterator: &mut std::slice::Iter<u8>) -> u16 {
    let byte = iterator.next().unwrap();
    return (*byte as u16) << 8 | word;
}

fn read_next_word(iterator: &mut std::slice::Iter<u8>) -> u16 {
    let lo = iterator.next().unwrap();
    let hi = iterator.next().unwrap();
    return (*hi as u16) << 8 | *lo as u16;
}

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
    debug!("    Mod: {:02b}", mod_field);
    debug!("    Reg: {:03b}", reg_field);
    debug!("    R/M: {:03b}", rm_field);

    let reg_field_map = REG_FIELD_ENCODING.get(&reg_field).unwrap();
    let reg_field_encoding = reg_field_map.get(&w_field).unwrap();
    debug!("    Reg encoding: {}", reg_field_encoding);

    let rm_field_map = REG_FIELD_ENCODING.get(&rm_field).unwrap();
    let mut rm_field_encoding = RM_FIELD_ENCODING.get(&rm_field).unwrap();
    debug!("    R/M encoding: {}", rm_field_encoding);

    let a = reg_field_encoding.to_string();
    let mut b = "".to_string();

    match mod_field {
        0b00 => {
            if rm_field_encoding.eq(&"BP") {
                let mut data = *iterator.next().unwrap() as u16;
                if w_field == 0b1 {
                    data = read_next_byte_and_combine(data, iterator);
                }
                b = format!("[{}]", data);
            } else {
                b = format!("[{}]", rm_field_encoding);
            }
        }
        0b01 => {
            let mut data = *iterator.next().unwrap();
            if w_field == 0b1 && rm_field_encoding.ne(&"BP") {
                data = !data + 0b1;
                b = format!("[{} - {}]", rm_field_encoding, data);
            } else {
                b = format!("[{} + {}]", rm_field_encoding, data);
            }
        }
        0b10 => {
            let mut data = read_next_word(iterator);
            if w_field == 0b1 {
                data = !data + 0b1;
                b = format!("[{} - {}]", rm_field_encoding, data);
            } else {
                b = format!("[{} + {}]", rm_field_encoding, data);
            }
        }
        0b11 => {
            rm_field_encoding = rm_field_map.get(&w_field).unwrap();
            debug!("    R/M encoding: {}", rm_field_encoding);
            b = rm_field_encoding.to_string();
        }
        _ => {
            panic!("Invalid mod field: {}", mod_field);
        }
    }
    if d_field == 0b1 {
        info!("MOV {}, {}", a, b);
    } else {
        info!("MOV {}, {}", b, a);
    }
}

fn decode_move_immediate_to_register_memory(byte: u8, iterator: &mut std::slice::Iter<u8>) {
    debug!("  MOV: Immediate to register/memory");

    const W_MASK: u8 = 0b0000_0001;
    let w_field = byte & W_MASK;
    debug!("    W: {:01b}", w_field);

    let next_byte = iterator.next().unwrap();
    debug!("  Next byte: {:08b}", next_byte);

    const MOD_MASK: u8 = 0b1100_0000;
    const RM_MASK: u8 = 0b0000_0111;
    let mod_field = (next_byte & MOD_MASK) >> 6;
    let rm_field = next_byte & RM_MASK;

    debug!("    Mod: {:02b}", mod_field);
    debug!("    R/M: {:03b}", rm_field);

    let rm_field_encoding = RM_FIELD_ENCODING.get(&rm_field).unwrap();
    debug!("    R/M encoding: {}", rm_field_encoding);

    match mod_field {
        0b00 => {
            let mut data = *iterator.next().unwrap() as u16;
            debug!("    Data1: {:08b}", data);
            if w_field == 0b1 {
                data = read_next_byte_and_combine(data, iterator);
            }
            info!("MOV [{}], byte {}", rm_field_encoding, data);
        }
        0b01 => {
            let disp_lo = iterator.next().unwrap();
            debug!("    disp_lo: {:08b}", disp_lo);
            let mut data = *iterator.next().unwrap() as u16;
            debug!("    Data1: {:08b}", data);
            if w_field == 0b1 {
                data = read_next_byte_and_combine(data, iterator);
            }
            info!("MOV [{} + {}], word {}", rm_field_encoding, disp_lo, data);
        }
        0b10 => {
            let disp = read_next_word(iterator);
            debug!("    disp: {:016b}", disp);
            let mut data = *iterator.next().unwrap() as u16;
            debug!("    Data1: {:08b}", data);
            if w_field == 0b1 {
                data = read_next_byte_and_combine(data, iterator);
                info!("MOV [{} + {}], word {}", rm_field_encoding, disp, data);
            } else {
                info!("MOV [{} + {}], byte {}", rm_field_encoding, disp, data);
            }
        }
        _ => {
            panic!("Invalid mod field: {}", mod_field);
        }
    }
}

fn decode_move_immediate_to_register(byte: u8, iterator: &mut std::slice::Iter<u8>) {
    debug!("  MOV: Immediate to register");

    const W_MASK: u8 = 0b0000_1000;
    const REG_MASK: u8 = 0b0000_0111;

    let w_field = (byte & W_MASK) >> 3;
    let reg_field = byte & REG_MASK;
    debug!("    W: {:01b}", w_field);
    debug!("    Reg: {:03b}", reg_field);

    let reg_field_map = REG_FIELD_ENCODING.get(&reg_field).unwrap();
    let reg_field_encoding = reg_field_map.get(&w_field).unwrap();
    debug!("    Reg encoding: {}", reg_field_encoding);

    let mut data = *iterator.next().unwrap() as u16;
    debug!("  data1: {:08b}", data);

    if w_field == 0b1 {
        data = read_next_byte_and_combine(data, iterator);
    }
    info!("MOV {}, {}", reg_field_encoding, data);
}

fn decode_move_memory_to_fro_accumulator(
    byte: u8,
    iterator: &mut std::slice::Iter<u8>,
    reverse: bool,
) {
    debug!("  MOV: Memory to/fro accumulator");

    const W_MASK: u8 = 0b0000_0001;
    let w_field = byte & W_MASK;
    debug!("    W: {:01b}", w_field);

    let mut addr = *iterator.next().unwrap() as u16;
    debug!("  addr_lo: {:08b}", addr);

    if w_field == 0b1 {
        addr = read_next_byte_and_combine(addr, iterator);
    }
    debug!("  addr: {:016b}", addr);
    if reverse {
        info!("MOV [{}], AX", addr);
    } else {
        info!("MOV AX, [{}]", addr);
    }
}

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

    while let Some(byte) = iterator.next() {
        debug!("Byte: {:08b}", byte);
        match byte {
            byte if (byte & MASK_SIX_BIT) == MOV_REG_MEM_TO_FRO_REG_OPCODE => {
                decode_move_register_memory_to_from_register(*byte, &mut iterator)
            }
            byte if (byte & MASK_SEVEN_BIT) == MOV_IMM_TO_REG_MEM_OPCODE => {
                decode_move_immediate_to_register_memory(*byte, &mut iterator)
            }
            byte if (byte & MASK_FOUR_BIT) == MOV_IMM_TO_REG_OPCODE => {
                decode_move_immediate_to_register(*byte, &mut iterator)
            }
            byte if (byte & MASK_SEVEN_BIT) == MOV_MEM_TO_ACC_OPCODE => {
                decode_move_memory_to_fro_accumulator(*byte, &mut iterator, false)
            }
            byte if (byte & MASK_SEVEN_BIT) == MOV_ACC_TO_MEM_OPCODE => {
                decode_move_memory_to_fro_accumulator(*byte, &mut iterator, true)
            }
            _ => {}
        }
    }
}
