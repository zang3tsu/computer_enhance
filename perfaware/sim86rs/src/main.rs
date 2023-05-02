use std::env;
use std::fmt::format;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;

use env_logger::{Builder, Target};
use lazy_static::lazy_static;
use log::{debug, error, info, trace, warn, Level, LevelFilter};

mod decoders;
use decoders::*;

const MASK_FOUR_BITS: u8 = 0b1111_0000;
const MASK_FIVE_BITS: u8 = 0b1111_1000;
const MASK_SIX_BITS: u8 = 0b1111_1100;
const MASK_SEVEN_BITS: u8 = 0b1111_1110;
const MASK_MIDDLE_TWO_BITS: u8 = 0b1110_0111;

const MOV_ACC_TO_MEM_OPCODE: u8 = 0b1010_0010;
const MOV_IMM_TO_REG_MEM_OPCODE: u8 = 0b1100_0110;
const MOV_IMM_TO_REG_OPCODE: u8 = 0b1011_0000;
const MOV_MEM_TO_ACC_OPCODE: u8 = 0b1010_0000;
const MOV_REG_MEM_TO_FRO_REG_OPCODE: u8 = 0b1000_1000;
const ADD_REG_MEM_WITH_REG_OPCODE: u8 = 0b0000_0000;
const ADD_IMM_TO_REG_MEM_OPCODE: u8 = 0b1000_0000;
const ADD_IMM_TO_ACC_OPCODE: u8 = 0b0000_0100;
const SUB_REG_MEM_WITH_REG_OPCODE: u8 = 0b0010_1000;
const SUB_IMM_FROM_ACC_OPCODE: u8 = 0b0010_1100;
const CMP_REG_MEM_AND_REG_OPCODE: u8 = 0b0011_1000;
const CMP_IMM_WITH_ACC_OPCODE: u8 = 0b0011_1100;
const JNZ_OPCODE: u8 = 0b0111_0101;
const JE_OPCODE: u8 = 0b0111_0100;
const JL_OPCODE: u8 = 0b0111_1100;
const JLE_OPCODE: u8 = 0b0111_1110;
const JB_OPCODE: u8 = 0b0111_0010;
const JBE_OPCODE: u8 = 0b0111_0110;
const JP_OPCODE: u8 = 0b0111_1010;
const JO_OPCODE: u8 = 0b0111_0000;
const JS_OPCODE: u8 = 0b0111_1000;
const JNE_OPCODE: u8 = 0b0111_0101;
const JNL_OPCODE: u8 = 0b0111_1101;
const JG_OPCODE: u8 = 0b0111_1111;
const JNB_OPCODE: u8 = 0b0111_0011;
const JA_OPCODE: u8 = 0b0111_0111;
const JNP_OPCODE: u8 = 0b0111_1011;
const JNO_OPCODE: u8 = 0b0111_0001;
const JNS_OPCODE: u8 = 0b0111_1001;
const LOOP_OPCODE: u8 = 0b1110_0010;
const LOOPZ_OPCODE: u8 = 0b1110_0001;
const LOOPNZ_OPCODE: u8 = 0b1110_0000;
const JCXZ_OPCODE: u8 = 0b1110_0011;
const PUSH_REG_MEM_OPCODE: u8 = 0b1111_1111;
const PUSH_REG_OPCODE: u8 = 0b0101_0000;
const PUSH_SEG_REG_OPCODE: u8 = 0b0000_0110;
const POP_REG_MEM_OPCODE: u8 = 0b1000_1111;
const POP_REG_OPCODE: u8 = 0b0101_1000;
const POP_SEG_REG_OPCODE: u8 = 0b0000_0111;
const XCHG_REG_MEM_WITH_REG_OPCODE: u8 = 0b1000_0110;

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
        debug!("Byte: {:08b} {}", byte, byte);
        match byte {
            byte if (byte & MASK_SIX_BITS) == MOV_REG_MEM_TO_FRO_REG_OPCODE => {
                decode_register_memory_to_from_register("MOV", *byte, &mut iterator)
            }
            byte if (byte & MASK_SEVEN_BITS) == MOV_IMM_TO_REG_MEM_OPCODE => {
                decode_immediate_to_register_memory("MOV", *byte, &mut iterator)
            }
            byte if (byte & MASK_FOUR_BITS) == MOV_IMM_TO_REG_OPCODE => {
                decode_immediate_to_register("MOV", *byte, &mut iterator)
            }
            byte if (byte & MASK_SEVEN_BITS) == MOV_MEM_TO_ACC_OPCODE => {
                decode_memory_to_fro_accumulator("MOV", *byte, &mut iterator, false)
            }
            byte if (byte & MASK_SEVEN_BITS) == MOV_ACC_TO_MEM_OPCODE => {
                decode_memory_to_fro_accumulator("MOV", *byte, &mut iterator, true)
            }
            byte if (byte & MASK_SIX_BITS) == ADD_REG_MEM_WITH_REG_OPCODE => {
                decode_register_memory_to_from_register("ADD", *byte, &mut iterator)
            }
            byte if (byte & MASK_SIX_BITS) == ADD_IMM_TO_REG_MEM_OPCODE => {
                decode_immediate_to_register_memory("ADD", *byte, &mut iterator)
            }
            byte if (byte & MASK_SEVEN_BITS) == ADD_IMM_TO_ACC_OPCODE => {
                decode_memory_to_fro_accumulator("ADD", *byte, &mut iterator, false)
            }
            byte if (byte & MASK_SIX_BITS) == SUB_REG_MEM_WITH_REG_OPCODE => {
                decode_register_memory_to_from_register("SUB", *byte, &mut iterator)
            }
            byte if (byte & MASK_SEVEN_BITS) == SUB_IMM_FROM_ACC_OPCODE => {
                decode_memory_to_fro_accumulator("SUB", *byte, &mut iterator, false)
            }
            byte if (byte & MASK_SIX_BITS) == CMP_REG_MEM_AND_REG_OPCODE => {
                decode_register_memory_to_from_register("CMP", *byte, &mut iterator)
            }
            byte if (byte & MASK_SEVEN_BITS) == CMP_IMM_WITH_ACC_OPCODE => {
                decode_memory_to_fro_accumulator("CMP", *byte, &mut iterator, false)
            }
            byte if byte == &JNZ_OPCODE => decode_jump("JNZ", *byte, &mut iterator),
            byte if byte == &JE_OPCODE => decode_jump("JE", *byte, &mut iterator),
            byte if byte == &JL_OPCODE => decode_jump("JL", *byte, &mut iterator),
            byte if byte == &JLE_OPCODE => decode_jump("JLE", *byte, &mut iterator),
            byte if byte == &JB_OPCODE => decode_jump("JB", *byte, &mut iterator),
            byte if byte == &JBE_OPCODE => decode_jump("JBE", *byte, &mut iterator),
            byte if byte == &JP_OPCODE => decode_jump("JP", *byte, &mut iterator),
            byte if byte == &JO_OPCODE => decode_jump("JO", *byte, &mut iterator),
            byte if byte == &JS_OPCODE => decode_jump("JS", *byte, &mut iterator),
            byte if byte == &JNE_OPCODE => decode_jump("JNE", *byte, &mut iterator),
            byte if byte == &JNL_OPCODE => decode_jump("JNL", *byte, &mut iterator),
            byte if byte == &JG_OPCODE => decode_jump("JG", *byte, &mut iterator),
            byte if byte == &JNB_OPCODE => decode_jump("JNB", *byte, &mut iterator),
            byte if byte == &JA_OPCODE => decode_jump("JA", *byte, &mut iterator),
            byte if byte == &JNP_OPCODE => decode_jump("JNP", *byte, &mut iterator),
            byte if byte == &JNO_OPCODE => decode_jump("JNO", *byte, &mut iterator),
            byte if byte == &JNS_OPCODE => decode_jump("JNS", *byte, &mut iterator),
            byte if byte == &LOOP_OPCODE => decode_jump("LOOP", *byte, &mut iterator),
            byte if byte == &LOOPZ_OPCODE => decode_jump("LOOPZ", *byte, &mut iterator),
            byte if byte == &LOOPNZ_OPCODE => decode_jump("LOOPNZ", *byte, &mut iterator),
            byte if byte == &JCXZ_OPCODE => decode_jump("JCXZ", *byte, &mut iterator),
            byte if byte == &PUSH_REG_MEM_OPCODE => {
                decode_register_memory_to_from_register("PUSH", *byte, &mut iterator)
            }
            byte if (byte & MASK_FIVE_BITS) == PUSH_REG_OPCODE => decode_register("PUSH", *byte),
            byte if (byte & MASK_MIDDLE_TWO_BITS) == PUSH_SEG_REG_OPCODE => {
                decode_segment_register("PUSH", *byte)
            }
            byte if byte == &POP_REG_MEM_OPCODE => {
                decode_register_memory_to_from_register("POP", *byte, &mut iterator)
            }
            byte if (byte & MASK_FIVE_BITS) == POP_REG_OPCODE => decode_register("POP", *byte),
            byte if (byte & MASK_MIDDLE_TWO_BITS) == POP_SEG_REG_OPCODE => {
                decode_segment_register("POP", *byte)
            }
            byte if (byte & MASK_SEVEN_BITS) == XCHG_REG_MEM_WITH_REG_OPCODE => {
                decode_register_memory_to_from_register("XCHG", *byte, &mut iterator)
            }
            _ => {
                panic!("No matching decode")
            }
        }
    }
}
