pub fn normalize_embedded_font_program(bytes: Vec<u8>) -> Vec<u8> {
    let Some((records_start, records_end, num_tables)) = sfnt_directory_bounds(&bytes) else {
        return bytes;
    };

    let mut records = Vec::with_capacity(num_tables);
    let mut cursor = records_start;
    while cursor + 16 <= records_end {
        let mut record = [0u8; 16];
        record.copy_from_slice(&bytes[cursor..cursor + 16]);
        records.push(record);
        cursor += 16;
    }

    if records.len() != num_tables || is_sorted_records(&records) {
        return bytes;
    }

    records.sort_by(|left, right| left[..4].cmp(&right[..4]));

    let mut normalized = bytes;
    let mut cursor = records_start;
    for record in records {
        normalized[cursor..cursor + 16].copy_from_slice(&record);
        cursor += 16;
    }
    normalized
}
fn sfnt_directory_bounds(bytes: &[u8]) -> Option<(usize, usize, usize)> {
    if bytes.len() < 12 {
        return None;
    }
    let signature = &bytes[..4];
    let is_sfnt = matches!(
        signature,
        [0x00, 0x01, 0x00, 0x00] | [b'O', b'T', b'T', b'O'] | [b't', b'r', b'u', b'e']
    );
    if !is_sfnt {
        return None;
    }

    let num_tables = u16::from_be_bytes([bytes[4], bytes[5]]) as usize;
    if num_tables == 0 {
        return None;
    }

    let records_start = 12usize;
    let records_len = num_tables.checked_mul(16)?;
    let records_end = records_start.checked_add(records_len)?;
    if records_end > bytes.len() {
        return None;
    }
    Some((records_start, records_end, num_tables))
}
fn is_sorted_records(records: &[[u8; 16]]) -> bool {
    records.windows(2).all(|pair| pair[0][..4] <= pair[1][..4])
}

#[cfg(test)]
mod tests {
    use super::normalize_embedded_font_program;

    #[test]
    fn sorts_sfnt_records() {
        let mut bytes = vec![
            0x00, 0x01, 0x00, 0x00, // sfnt
            0x00, 0x02, // numTables
            0x00, 0x10, 0x00, 0x01, 0x00, 0x20, // searchRange/entrySelector/rangeShift
        ];
        bytes.extend_from_slice(b"maxp\x00\x00\x00\x00\x00\x00\x00 \x00\x00\x00\x10");
        bytes.extend_from_slice(b"head\x00\x00\x00\x00\x00\x00\x000\x00\x00\x00\x10");
        bytes.extend_from_slice(&[0xAA; 0x40]);

        let normalized = normalize_embedded_font_program(bytes.clone());

        assert_eq!(&normalized[12..16], b"head");
        assert_eq!(&normalized[28..32], b"maxp");
        assert_eq!(&normalized[44..], &bytes[44..]);
    }
}
