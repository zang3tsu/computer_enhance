use log::{debug, error, info};

use crate::decoders::{
    decode_far_proc_label, decode_immed16, decode_immed8, decode_immediate_to_register,
    decode_immediate_to_register_memory, decode_jump, decode_memory_to_fro_accumulator,
    decode_near_proc_label, decode_register, decode_register_memory_to_from_register,
    decode_repeat, decode_segment_register,
};

pub fn decode_first_byte(byte: u8, iterator: &mut std::slice::Iter<u8>) {
    debug!("First Byte: 0b{:08b} 0x{:02x}", byte, byte);
    match byte {
        0x0..=0x3 => decode_register_memory_to_from_register("ADD", byte, iterator),
        0x4..=0x5 => decode_memory_to_fro_accumulator("ADD", byte, iterator, false),
        0x6 => decode_segment_register("PUSH", byte),
        0x8..=0x0B => decode_register_memory_to_from_register("OR", byte, iterator),
        0x0C..=0x0D => decode_memory_to_fro_accumulator("OR", byte, iterator, false),
        0x0E => decode_segment_register("PUSH", byte),
        0x10..=0x13 => decode_register_memory_to_from_register("ADC", byte, iterator),
        0x14..=0x15 => decode_memory_to_fro_accumulator("ADC", byte, iterator, false),
        0x18..=0x1B => decode_register_memory_to_from_register("SBB", byte, iterator),
        0x1C..=0x1D => decode_memory_to_fro_accumulator("SBB", byte, iterator, false),
        0x1F => decode_segment_register("POP", byte),
        0x20..=0x23 => decode_register_memory_to_from_register("AND", byte, iterator),
        0x24..=0x25 => decode_memory_to_fro_accumulator("AND", byte, iterator, false),
        0x26 => info!("ES"),
        0x27 => info!("DAA"),
        0x28..=0x2B => decode_register_memory_to_from_register("SUB", byte, iterator),
        0x2C..=0x2D => decode_memory_to_fro_accumulator("SUB", byte, iterator, false),
        0x2E => info!("CS"),
        0x2F => info!("DAS"),
        0x30..=0x33 => decode_register_memory_to_from_register("XOR", byte, iterator),
        0x34..=0x35 => decode_memory_to_fro_accumulator("XOR", byte, iterator, false),
        0x36 => info!("SS"),
        0x37 => info!("AAA"),
        0x38..=0x3B => decode_register_memory_to_from_register("CMP", byte, iterator),
        0x3C..=0x3D => decode_memory_to_fro_accumulator("CMP", byte, iterator, false),
        0x3E => info!("DS"),
        0x3F => info!("AAS"),
        0x40..=0x47 => decode_register("INC", byte),
        0x48..=0x4F => decode_register("DEC", byte),
        0x50..=0x57 => decode_register("PUSH", byte),
        0x58..=0x5F => decode_register("POP", byte),
        0x70 => decode_jump("JO", byte, iterator),
        0x71 => decode_jump("JNO", byte, iterator),
        0x72 => decode_jump("JB", byte, iterator),
        0x73 => decode_jump("JAE", byte, iterator),
        0x74 => decode_jump("JZ", byte, iterator),
        0x75 => decode_jump("JNZ", byte, iterator),
        0x76 => decode_jump("JBE", byte, iterator),
        0x77 => decode_jump("JA", byte, iterator),
        0x78 => decode_jump("JS", byte, iterator),
        0x79 => decode_jump("JNS", byte, iterator),
        0x7A => decode_jump("JP", byte, iterator),
        0x7B => decode_jump("JNP", byte, iterator),
        0x7C => decode_jump("JL", byte, iterator),
        0x7D => decode_jump("JGE", byte, iterator),
        0x7E => decode_jump("JLE", byte, iterator),
        0x7F => decode_jump("JG", byte, iterator),
        0x80..=0x83 => decode_immediate_to_register_memory("ADD", byte, iterator),
        0x84..=0x85 => decode_register_memory_to_from_register("TEST", byte, iterator),
        0x86 => decode_register_memory_to_from_register("XCHG", byte, iterator),
        0x87 => decode_register_memory_to_from_register("XCHG", byte, iterator),
        0x88..=0x8B => decode_register_memory_to_from_register("MOV", byte, iterator),
        0x8C => decode_register_memory_to_from_register("MOV", byte, iterator),
        0x8D => decode_register_memory_to_from_register("LEA", byte, iterator),
        0x8F => decode_register_memory_to_from_register("POP", byte, iterator),
        0x90 => info!("NOP"),
        0x92 => info!("XCHG AX, DX"),
        0x94 => info!("XCHG AX, SP"),
        0x96 => info!("XCHG AX, SI"),
        0x97 => info!("XCHG AX, DI"),
        0x98 => info!("CBW"),
        0x99 => info!("CWD"),
        0x9A => decode_far_proc_label("CALL", byte, iterator),
        0x9B => info!("WAIT"),
        0x9C => info!("PUSHF"),
        0x9D => info!("POPF"),
        0x9E => info!("SAHF"),
        0x9F => info!("LAHF"),
        0xA0..=0xA1 => decode_memory_to_fro_accumulator("MOV", byte, iterator, false),
        0xA2..=0xA3 => decode_memory_to_fro_accumulator("MOV", byte, iterator, true),
        0xA8..=0xA9 => decode_memory_to_fro_accumulator("TEST", byte, iterator, false),
        0xB0..=0xBF => decode_immediate_to_register("MOV", byte, iterator),
        0xC2 => decode_immediate_to_register("RET", byte, iterator),
        0xC3 => info!("RET"),
        0xC4 => decode_register_memory_to_from_register("LES", byte, iterator),
        0xC5 => decode_register_memory_to_from_register("LDS", byte, iterator),
        0xC6..=0xC7 => decode_immediate_to_register_memory("MOV", byte, iterator),
        0xCA => decode_immed16("RETF", byte, iterator),
        0xCB => info!("RETF"),
        0xCC => info!("INT3"),
        0xCD => decode_immed8("INT", byte, iterator),
        0xCE => info!("INTO"),
        0xCF => info!("IRET"),
        0xD0..=0xD3 => decode_register_memory_to_from_register("SHL", byte, iterator),
        0xD4 => {
            info!("AAM");
            iterator.next();
        }
        0xD5 => {
            info!("AAD");
            iterator.next();
        }
        0xD7 => info!("XLAT"),
        0xE0 => decode_jump("LOOPNZ", byte, iterator),
        0xE1 => decode_jump("LOOPZ", byte, iterator),
        0xE2 => decode_jump("LOOP", byte, iterator),
        0xE3 => decode_jump("JCXZ", byte, iterator),
        0xE4..=0xE5 => decode_memory_to_fro_accumulator("IN", byte, iterator, false),
        0xE8 => decode_near_proc_label("CALL", byte, iterator),
        0xE9 => decode_near_proc_label("JMP", byte, iterator),
        0xEA => decode_far_proc_label("JMP", byte, iterator),
        0xEC => info!("IN AL, DX"),
        0xED => info!("IN AX, DX"),
        0xEE => info!("OUT DX, AL"),
        0xE6..=0xE7 => decode_memory_to_fro_accumulator("OUT", byte, iterator, true),
        0xF0 => info!("LOCK"),
        0xF3 => decode_repeat("REP", byte, iterator),
        0xF4 => info!("HLT"),
        0xF5 => info!("CMC"),
        0xF6..=0xF7 => decode_register_memory_to_from_register("NEG", byte, iterator),
        0xF8 => info!("CLC"),
        0xF9 => info!("STC"),
        0xFA => info!("CLI"),
        0xFB => info!("STI"),
        0xFC => info!("CLD"),
        0xFD => info!("STD"),
        0xFE => decode_register_memory_to_from_register("INC", byte, iterator),
        0xFF => decode_register_memory_to_from_register("PUSH", byte, iterator),
        _ => {
            error!("Unknown opcode: 0b{:08b} 0x{:02x}", byte, byte);
            std::process::exit(1);
        }
    }
}
