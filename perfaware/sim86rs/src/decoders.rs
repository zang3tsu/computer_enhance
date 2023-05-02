use lazy_static::lazy_static;
use log::{debug, error, info, trace, warn, Level, LevelFilter};
use std::collections::HashMap;

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

fn read_next_byte_and_combine(word: u16, iterator: &mut std::slice::Iter<u8>) -> u16 {
    let byte = iterator.next().unwrap();
    return (*byte as u16) << 8 | word;
}

fn read_next_word(iterator: &mut std::slice::Iter<u8>) -> u16 {
    let lo = iterator.next().unwrap();
    let hi = iterator.next().unwrap();
    return (*hi as u16) << 8 | *lo as u16;
}

pub fn decode_register_memory_to_from_register(
    op: &str,
    byte: u8,
    iterator: &mut std::slice::Iter<u8>,
) {
    debug!("  {}: Register/memory to/from register", op);

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
            debug!("    Data: {:08b} {}", data, data);
            if w_field == 0b1 && rm_field_encoding.ne(&"BP") && ["MOV", "PUSH", "POP"].contains(&op)
            {
                data = !data + 0b1;
                b = format!("[{} - {}]", rm_field_encoding, data);
            } else {
                b = format!("[{} + {}]", rm_field_encoding, data);
            }
            // let extra = iterator.next().unwrap();
            // debug!("    Extra: {:08b} {}", extra, extra);
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
    if ["PUSH", "POP"].contains(&op) {
        info!("{} WORD {}", op, b);
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
        0b101 => {
            op = "SUB";
        }
        0b111 => {
            op = "CMP";
        }
        _ => {}
    }
    debug!("    Op: {}", op);

    let mut rm_field_encoding = RM_FIELD_ENCODING.get(&rm_field).unwrap();
    debug!("    R/M encoding: {}", rm_field_encoding);

    match mod_field {
        0b01 => {
            let disp_lo = iterator.next().unwrap();
            debug!("    disp_lo: {:08b}", disp_lo);
            let mut data = *iterator.next().unwrap() as u16;
            debug!("    Data1: {:08b}", data);
            if w_field == 0b1 {
                data = read_next_byte_and_combine(data, iterator);
            }
            info!(
                "{} [{} + {}], WORD {}",
                op, rm_field_encoding, disp_lo, data
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
            rm_field_encoding = rm_field_map.get(&w_field).unwrap();
            debug!("    R/M encoding: {}", rm_field_encoding);
            let mut data = *iterator.next().unwrap() as u16;
            debug!("    data: {:16b}", data);
            // if w_field == 0b1 {
            //     data = read_next_byte_and_combine(data, iterator);
            //     debug!("    data: {:16b}", data);
            // }
            info!("{} {}, {}", op, rm_field_encoding, data);
        }
        0b00 => {
            let mut addr = format!("[{}]", rm_field_encoding);
            if rm_field_encoding.eq(&"BP") {
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

    let mut data = *iterator.next().unwrap() as u16;
    debug!("  data1: {:08b}", data);

    if w_field == 0b1 {
        data = read_next_byte_and_combine(data, iterator);
    }
    info!("{} {}, {}", op, reg_field_encoding, data);
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
        data = read_next_byte_and_combine(data, iterator);
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
