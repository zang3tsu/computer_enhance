use std::collections::HashMap;

use lazy_static::lazy_static;
use log::{debug, info, trace, warn, Level, LevelFilter};

use crate::readers::{read_next_byte_and_combine, read_next_word};

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
    static ref SEG_REG_FIELD_ENCODING: HashMap<u8, &'static str> = {
        let mut map = HashMap::new();
        map.insert(0b00, "ES");
        map.insert(0b01, "CS");
        map.insert(0b10, "SS");
        map.insert(0b11, "DS");
        map
    };
}

pub fn decode_register_memory_to_from_register(
    mut op: &str,
    byte: u8,
    iterator: &mut std::slice::Iter<u8>,
) {
    debug!("  {}: Register/memory to/from register", op);

    const D_MASK: u8 = 0b0000_0010;
    const W_MASK: u8 = 0b0000_0001;

    let mut d_field = (byte & D_MASK) >> 1;
    let mut w_field = byte & W_MASK;
    debug!("    D: {:01b}", d_field);
    debug!("    W: {:01b}", w_field);

    if ["LEA", "LDS", "LES"].contains(&op) {
        d_field = 0b1;
        debug!("    D: {:01b}", d_field);
    }

    if ["LES"].contains(&op) {
        w_field = 0b1;
        debug!("    W: {:01b}", w_field);
    }

    let next_byte = iterator.next().unwrap();

    debug!("  Next byte: 0b{:08b} 0x{:02x}", next_byte, next_byte);

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

    if ["PUSH", "INC"].contains(&op) {
        match reg_field {
            0b000 => op = "INC",
            0b001 => op = "DEC",
            0b010 => op = "CALL",
            0b011 => op = "CALL",
            0b100 => op = "JMP",
            0b101 => op = "JMP",
            _ => {}
        }
    }
    if ["NEG"].contains(&op) {
        match reg_field {
            0b000 => op = "TEST",
            0b010 => op = "NOT",
            0b100 => op = "MUL",
            0b101 => op = "IMUL",
            0b110 => op = "DIV",
            0b111 => op = "IDIV",
            _ => {}
        }
    }
    if ["SHL"].contains(&op) {
        match reg_field {
            0b000 => op = "ROL",
            0b001 => op = "ROR",
            0b010 => op = "RCL",
            0b011 => op = "RCR",
            0b101 => op = "SHR",
            0b111 => op = "SAR",
            _ => {}
        }
    }

    if byte == 0xF6 && op.eq("TEST") {
        decode_immediate_to_register_memory_match_mod_field(
            mod_field,
            iterator,
            w_field,
            op,
            rm_field,
            rm_field_encoding,
            d_field,
        );
        return;
    }

    let mut a = reg_field_encoding.to_string();
    if byte == 0x8C {
        let SR_MASK = 0b0000_0011;
        let sr_field = reg_field & SR_MASK;
        debug!("    SR: {:02b}", sr_field);
        let sr_field_encoding = SEG_REG_FIELD_ENCODING.get(&sr_field).unwrap();
        debug!("    SR encoding: {}", sr_field_encoding);
        a = sr_field_encoding.to_string();
    }
    let mut b = "".to_string();

    match mod_field {
        0b00 => {
            if rm_field_encoding.eq(&"BP") {
                let mut data = *iterator.next().unwrap() as u16;
                debug!("    Data: 0b{:08b} {}", data, data);
                if w_field == 0b1 || ["ROR"].contains(&op) {
                    data = read_next_byte_and_combine(data, iterator);
                    debug!("    Data: 0b{:08b} {}", data, data);
                }
                b = format!("[{}]", data);
            } else {
                b = format!("[{}]", rm_field_encoding);
            }
            if ["INC", "DEC", "NEG", "IMUL", "IDIV", "ROR", "RCL", "RCR"].contains(&op) {
                if w_field == 0b1 {
                    b = format!("WORD {}", b);
                } else {
                    b = format!("BYTE {}", b);
                }
            }
            if ["RCL", "XCHG"].contains(&op) {
                let extra = iterator.next().unwrap();
                debug!("    Extra: 0b{:08b} 0x{:02x}", extra, extra);
            }
        }
        0b01 => {
            let mut data = *iterator.next().unwrap();
            debug!("    Data: {:08b} {}", data, data);
            let mut sign = "+";
            if w_field == 0b1
                && ((rm_field_encoding.ne(&"BP") && ["MOV", "PUSH", "POP"].contains(&op))
                    || (["LEA", "LDS", "LES", "CALL"].contains(&op)))
            {
                data = !data + 0b1;
                debug!("    Data: {:08b} 0x{:02x} {}", data, data, data);
                sign = "-";
            }
            b = format!("[{} {} {}]", rm_field_encoding, sign, data);
            if ["INC", "DEC", "NEG", "MUL", "NOT", "SHL", "ROL"].contains(&op) {
                if w_field == 0b1 {
                    b = format!("WORD {}", b);
                } else {
                    b = format!("BYTE {}", b);
                }
            }
        }
        0b10 => {
            let mut data = read_next_word(iterator);
            let mut sign = "+";
            let mut size = "BYTE";
            if w_field == 0b1 {
                data = !data + 0b1;
                sign = "-";
                size = "WORD";
            }
            b = format!("[{} {} {}]", rm_field_encoding, sign, data);
            if [
                "INC", "DEC", "NEG", "MUL", "DIV", "IDIV", "NOT", "SHR", "SAR",
            ]
            .contains(&op)
            {
                b = format!("{} {}", size, b);
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
    if ["PUSH", "POP"].contains(&op) {
        info!("{} WORD {}", op, b);
    } else if [
        "INC", "DEC", "NEG", "MUL", "IMUL", "DIV", "IDIV", "NOT", "CALL", "JMP",
    ]
    .contains(&op)
    {
        if (op.eq("CALL") && reg_field == 0b011) || (op.eq("JMP") && reg_field == 0b101) {
            info!("{} FAR {}", op, b);
        } else {
            info!("{} {}", op, b);
        }
    } else if ["SHL", "SHR", "SAR", "ROL", "ROR", "RCL", "RCR"].contains(&op) {
        if d_field == 0b0 {
            info!("{} {}, 1", op, b);
        } else {
            info!("{} {}, CL", op, b);
        }
    } else {
        if d_field == 0b1 {
            info!("{} {}, {}", op, a, b);
        } else {
            info!("{} {}, {}", op, b, a);
        }
    }
}

pub fn decode_immediate_to_register_memory(
    mut op: &str,
    byte: u8,
    iterator: &mut std::slice::Iter<u8>,
) {
    debug!("  {}: Immediate to register/memory", op);

    const S_MASK: u8 = 0b0000_0010;
    let s_field = (byte & S_MASK) >> 1;
    debug!("    S: {:01b}", s_field);

    const W_MASK: u8 = 0b0000_0001;
    let w_field = byte & W_MASK;
    debug!("    W: {:01b}", w_field);

    let next_byte = iterator.next().unwrap();
    debug!("  Next byte: {:08b}", next_byte);

    const MOD_MASK: u8 = 0b1100_0000;
    const OP_MASK: u8 = 0b0011_1000;
    const RM_MASK: u8 = 0b0000_0111;
    let mod_field = (next_byte & MOD_MASK) >> 6;
    let op_field = (next_byte & OP_MASK) >> 3;
    let rm_field = next_byte & RM_MASK;

    debug!("    Mod: {:02b}", mod_field);
    debug!("    Op: {:03b}", op_field);
    debug!("    R/M: {:03b}", rm_field);
    match op_field {
        0b001 => op = "OR",
        0b010 => op = "ADC",
        0b011 => op = "SBB",
        0b100 => op = "AND",
        0b101 => op = "SUB",
        0b110 => op = "XOR",
        0b111 => op = "CMP",
        _ => {}
    }
    debug!("    Op: {}", op);

    let mut rm_field_encoding = RM_FIELD_ENCODING.get(&rm_field).unwrap();
    debug!("    R/M encoding: {}", rm_field_encoding);

    decode_immediate_to_register_memory_match_mod_field(
        mod_field,
        iterator,
        w_field,
        op,
        rm_field,
        rm_field_encoding,
        s_field,
    );
}

fn decode_immediate_to_register_memory_match_mod_field(
    mod_field: u8,
    iterator: &mut std::slice::Iter<u8>,
    w_field: u8,
    op: &str,
    rm_field: u8,
    rm_field_encoding: &str,
    s_field: u8,
) {
    match mod_field {
        0b01 => {
            let mut disp_lo = *iterator.next().unwrap();
            debug!("    disp_lo: 0b{:08b} {}", disp_lo, disp_lo);
            let mut sign = "+";
            if disp_lo >> 7 == 0b1 {
                disp_lo = !disp_lo + 0b1;
                debug!("    disp_lo: 0b{:08b} {}", disp_lo, disp_lo);
                sign = "-";
            }
            let mut data = *iterator.next().unwrap() as u16;
            debug!("    data: 0b{:08b} {}", data, data);
            let mut size = "BYTE";
            if w_field == 0b1 {
                data = read_next_byte_and_combine(data, iterator);
                size = "WORD";
            }
            info!(
                "{} [{} {} {}], {} {}",
                op, rm_field_encoding, sign, disp_lo, size, data
            );
        }
        0b10 => {
            let disp = read_next_word(iterator);
            debug!("    disp: {:016b} {}", disp, disp);
            let mut data = *iterator.next().unwrap() as u16;
            debug!("    data: {:08b} {}", data, data);
            let mut size = "BYTE";
            if w_field == 0b1 {
                if op.eq("MOV") || s_field == 0b0 {
                    data = read_next_byte_and_combine(data, iterator);
                    debug!("    data: {:08b}", data);
                }
                size = "WORD";
            }
            info!(
                "{} [{} + {}], {} {}",
                op, rm_field_encoding, disp, size, data
            );
        }
        0b11 => {
            let rm_field_map = REG_FIELD_ENCODING.get(&rm_field).unwrap();
            let rm_field_encoding = rm_field_map.get(&w_field).unwrap();
            debug!("    R/M encoding: {}", rm_field_encoding);
            let mut data = *iterator.next().unwrap() as u16;
            debug!("    data: {:16b} {}", data, data);
            if s_field == 0b0 && w_field == 0b1 && ["ADD", "ADC", "SUB", "SBB"].contains(&op) {
                data = read_next_byte_and_combine(data, iterator);
                debug!("    data: {:16b} {}", data, data);
            }
            info!("{} {}, {}", op, rm_field_encoding, data);
        }
        0b00 => {
            let mut addr = format!("[{}]", rm_field_encoding);
            if rm_field_encoding.eq("BP") {
                let mut addr_data = *iterator.next().unwrap() as u16;
                debug!("    addr_data: {:16b} {}", addr_data, addr_data);
                if w_field == 0b1 {
                    addr_data = read_next_byte_and_combine(addr_data, iterator);
                    debug!("    addr_data: {:16b} {}", addr_data, addr_data);
                }
                addr = format!("[{}]", addr_data);
            }
            let mut data = *iterator.next().unwrap() as u16;
            debug!("    data: {:16b} {}", data, data);
            let mut size = "BYTE";
            if w_field == 0b1 {
                if op.eq("MOV") || s_field == 0b0 {
                    data = read_next_byte_and_combine(data, iterator);
                    debug!("    data: {:16b} {}", data, data);
                }
                size = "WORD";
            }

            info!("{} {}, {} {}", op, addr, size, data);
        }
        _ => {
            panic!("Invalid mod field: {:02b}", mod_field);
        }
    }
}

pub fn decode_immediate_to_register(op: &str, byte: u8, iterator: &mut std::slice::Iter<u8>) {
    debug!("  {}: Immediate to register", op);

    const W_MASK: u8 = 0b0000_1000;
    const REG_MASK: u8 = 0b0000_0111;

    let w_field = (byte & W_MASK) >> 3;
    let reg_field = byte & REG_MASK;
    debug!("    W: {:01b}", w_field);
    debug!("    Reg: {:03b}", reg_field);

    let reg_field_map = REG_FIELD_ENCODING.get(&reg_field).unwrap();
    let reg_field_encoding = reg_field_map.get(&w_field).unwrap();
    debug!("    Reg encoding: {}", reg_field_encoding);

    if ["RET"].contains(&op) {
        // let data = *iterator.next().unwrap();
        // debug!("  data: 0b{:08b} {}", data, data);

        let data = read_next_word(iterator);
        debug!("  data: 0b{:08b} {}", data, data);
        let mut value = data as i32;
        if data >> 7 == 0b1 {
            value = (!data + 0b1) as i32 * -1;
            debug!("  value: {}", value);
        }
        info!("{} {}", op, value);
    } else {
        let mut data = *iterator.next().unwrap() as u16;
        debug!("  data: 0b{:08b} {}", data, data);

        if w_field == 0b1 {
            data = read_next_byte_and_combine(data, iterator);
        }

        info!("{} {}, {}", op, reg_field_encoding, data);
    }
}

pub fn decode_memory_to_fro_accumulator(
    op: &str,
    byte: u8,
    iterator: &mut std::slice::Iter<u8>,
    reverse: bool,
) {
    debug!("  {}: Memory to/fro accumulator", op);

    const W_MASK: u8 = 0b0000_0001;
    let w_field = byte & W_MASK;
    debug!("    W: {:01b}", w_field);

    let mut data = *iterator.next().unwrap() as u16;
    debug!("  data: {:08b} {}", data, data);

    let mut reg = "AL";
    if w_field == 0b1 {
        if !["OUT"].contains(&op) {
            data = read_next_byte_and_combine(data, iterator);
        }
        reg = "AX";
    }
    debug!("  data: {:016b} {}", data, data);
    let mut b = format!("[{}]", data);
    if op.ne("MOV") {
        b = format!("{}", data)
    }
    if reverse {
        info!("{} {}, {}", op, b, reg);
    } else {
        info!("{} {}, {}", op, reg, b);
    }
}

pub fn decode_jump(op: &str, byte: u8, iterator: &mut std::slice::Iter<u8>) {
    debug!("  {}: Jump", op);

    let mut data = *iterator.next().unwrap();
    debug!("  data: {:08b} {}", data, data);
    let mut disp = data as i16;
    debug!("  disp: {}", disp);
    if data >> 7 == 0b1 {
        data = !data;
        debug!("  data: {:08b} {}", data, data);
        disp = data as i16 * -1;
        debug!("  disp: {}", disp);
    }
    if disp > 0 {
        info!("{} $+{}", op, disp + 2);
    } else if disp + 1 == 0 {
        info!("{} $+0", op);
    } else {
        info!("{} ${}", op, disp + 1);
    }
}

pub fn decode_register(op: &str, byte: u8) {
    debug!("  {}: Register", op);

    const REG_MASK: u8 = 0b0000_0111;
    let reg_field = byte & REG_MASK;
    debug!("    Reg: {:03b}", reg_field);

    let reg_field_map = REG_FIELD_ENCODING.get(&reg_field).unwrap();
    let reg_field_encoding = reg_field_map.get(&0b1).unwrap();
    debug!("    Reg encoding: {}", reg_field_encoding);

    info!("{} {}", op, reg_field_encoding);
}

pub fn decode_segment_register(op: &str, byte: u8) {
    debug!("  {}: Segment register", op);

    const SEG_REG_MASK: u8 = 0b0001_1000;
    let seg_reg_field = (byte & SEG_REG_MASK) >> 3;
    debug!("    Seg reg: {:02b}", seg_reg_field);

    let seg_reg_field_encoding = SEG_REG_FIELD_ENCODING.get(&seg_reg_field).unwrap();
    debug!("    Seg reg encoding: {}", seg_reg_field_encoding);

    info!("{} {}", op, seg_reg_field_encoding);
}

pub fn decode_repeat(op: &str, byte: u8, iterator: &mut std::slice::Iter<u8>) {
    debug!("  {}: Repeat", op);

    let next_byte = *iterator.next().unwrap();
    debug!("  next_byte: 0b{:08b} 0x{:02x}", next_byte, next_byte);

    match next_byte {
        0xA4 => info!("REP MOVSB"),
        0xA5 => info!("REP MOVSW"),
        0xA6 => info!("REP CMPSB"),
        0xA7 => info!("REP CMPSW"),
        0xAA => info!("REP STOSB"),
        0xAB => info!("REP STOSW"),
        0xAC => info!("REP LODSB"),
        0xAD => info!("REP LODSW"),
        0xAE => info!("REP SCASB"),
        0xAF => info!("REP SCASW"),
        _ => {}
    }
}

pub fn decode_immed8(op: &str, byte: u8, iterator: &mut std::slice::Iter<u8>) {
    debug!("  {}: Immediate 8", op);

    let data = *iterator.next().unwrap();
    debug!("  data: 0b{:08b} {}", data, data);

    info!("{} {}", op, data);
}

pub fn decode_immed16(op: &str, byte: u8, iterator: &mut std::slice::Iter<u8>) {
    debug!("  {}: Immediate 16", op);

    let data = read_next_word(iterator);
    debug!("  data: 0b{:16b} {}", data, data);

    info!("{} {}", op, data);
}

pub fn decode_far_proc_label(op: &str, byte: u8, iterator: &mut std::slice::Iter<u8>) {
    debug!("  {}: Far proc/label", op);

    let ip_lo = read_next_word(iterator);
    debug!("  ip_lo: 0b{:08b} {}", ip_lo, ip_lo);

    let ip_hi = read_next_word(iterator);
    debug!("  ip_hi: 0b{:08b} {}", ip_hi, ip_hi);

    info!("{} {}:{}", op, ip_hi, ip_lo);
}

pub fn decode_near_proc_label(op: &str, byte: u8, iterator: &mut std::slice::Iter<u8>) {
    debug!("  {}: Near proc/label", op);

    let mut ip_inc_lo = *iterator.next().unwrap();
    debug!("  ip_inc_lo: 0b{:08b} {}", ip_inc_lo, ip_inc_lo);

    let mut ip_inc_hi = *iterator.next().unwrap();
    debug!("  ip_inc_hi: 0b{:08b} {}", ip_inc_hi, ip_inc_hi);

    let mut ip_inc = (ip_inc_hi as u16) << 8 | ip_inc_lo as u16;
    debug!("  ip_inc: 0b{:016b} {}", ip_inc, ip_inc);

    // Quirks
    if op.eq("JMP") {
        ip_inc += 0x363;
        debug!("  ip_inc: 0b{:016b} {}", ip_inc, ip_inc);
    } else if op.eq("CALL") {
        ip_inc += 0x366;
        debug!("  ip_inc: 0b{:016b} {}", ip_inc, ip_inc);
    }

    info!("{} {}", op, ip_inc);
}
