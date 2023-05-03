pub fn read_next_byte_and_combine(word: u16, iterator: &mut std::slice::Iter<u8>) -> u16 {
    let byte = iterator.next().unwrap();
    return (*byte as u16) << 8 | word;
}

pub fn read_next_word(iterator: &mut std::slice::Iter<u8>) -> u16 {
    let lo = iterator.next().unwrap();
    let hi = iterator.next().unwrap();
    return (*hi as u16) << 8 | *lo as u16;
}
