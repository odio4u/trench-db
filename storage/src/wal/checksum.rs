/// CRC-32 lookup table (IEEE 802.3 polynomial).
fn crc32_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    for i in 0..256 {
        let mut crc = i as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = 0xEDB88320 ^ (crc >> 1);
            } else {
                crc >>= 1;
            }
        }
        table[i] = crc;
    }
    table
}

/// Computes the CRC-32 checksum of `data`.
pub fn crc32(data: &[u8]) -> u32 {
    let table = crc32_table();
    let mut crc: u32 = 0xFFFFFFFF;
    for byte in data {
        crc = table[((crc ^ *byte as u32) & 0xFF) as usize] ^ (crc >> 8);
    }
    crc ^ 0xFFFFFFFF
}
