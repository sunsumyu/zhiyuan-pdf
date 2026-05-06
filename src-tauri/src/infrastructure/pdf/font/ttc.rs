pub fn extract_ttc_face_as_ttf(data: &[u8], face_index: u32) -> Result<Vec<u8>, String> {
    let ttc_tag = data.get(0..4).ok_or("TTC too short for tag")?;
    if ttc_tag != b"ttcf" {
        return Err(format!(
            "expected TTC tag 'ttcf', got '{}',",
            String::from_utf8_lossy(ttc_tag)
        ));
    }
    let num_fonts = u32::from_be_bytes(
        data.get(8..12)
            .ok_or("TTC too short for numFonts")?
            .try_into()
            .map_err(|_| "invalid numFonts")?,
    ) as usize;
    if face_index as usize >= num_fonts {
        return Err(format!(
            "face_index {} >= num_fonts {}",
            face_index, num_fonts
        ));
    }
    let offset_table_offset = 12 + face_index as usize * 4;
    let font_offset = u32::from_be_bytes(
        data.get(offset_table_offset..offset_table_offset + 4)
            .ok_or("TTC too short for offset")?
            .try_into()
            .map_err(|_| "invalid offset")?,
    ) as usize;

    let num_tables = u16::from_be_bytes(
        data.get(font_offset + 4..font_offset + 6)
            .ok_or("font too short for numTables")?
            .try_into()
            .map_err(|_| "invalid numTables")?,
    ) as usize;

    let (search_range, entry_selector, range_shift) = sfnt_search_params(num_tables as u16);

    let mut output = Vec::new();
    output.extend_from_slice(&data[font_offset..font_offset + 12]);
    output.extend_from_slice(&search_range.to_be_bytes());
    output.extend_from_slice(&entry_selector.to_be_bytes());
    output.extend_from_slice(&range_shift.to_be_bytes());

    let mut record_meta: Vec<SfntTableRecord> = Vec::with_capacity(num_tables);
    let mut offset = 12 + num_tables * 16;

    for i in 0..num_tables {
        let base = font_offset + 12 + i * 16;
        let tag = data
            .get(base..base + 4)
            .ok_or("table record too short")?
            .try_into()
            .map_err(|_| "invalid tag")?;
        let table_offset = u32::from_be_bytes(
            data.get(base + 8..base + 12)
                .ok_or("table record too short")?
                .try_into()
                .map_err(|_| "invalid offset")?,
        ) as usize;
        let length = u32::from_be_bytes(
            data.get(base + 12..base + 16)
                .ok_or("table record too short")?
                .try_into()
                .map_err(|_| "invalid length")?,
        ) as usize;

        let table_data = data
            .get(table_offset..table_offset + length)
            .ok_or("table data out of bounds")?;

        let padded_len = align4(length);
        record_meta.push(SfntTableRecord {
            tag,
            checksum: checksum(table_data),
            offset,
            length,
        });
        output.extend_from_slice(table_data);
        if padded_len > length {
            output.resize(offset + padded_len, 0);
        }
        offset += padded_len;
    }

    for (idx, record) in record_meta.iter().enumerate() {
        let base = 12 + idx * 16;
        output[base..base + 4].copy_from_slice(&record.tag);
        output[base + 4..base + 8].copy_from_slice(&record.checksum.to_be_bytes());
        output[base + 8..base + 12].copy_from_slice(&(record.offset as u32).to_be_bytes());
        output[base + 12..base + 16].copy_from_slice(&(record.length as u32).to_be_bytes());
    }

    if let Some(record) = record_meta.iter().find(|r| r.tag == *b"head") {
        let total = checksum(&output);
        let adjustment = 0xB1B0AFBAu32.wrapping_sub(total);
        let start = record.offset + 8;
        if start + 4 <= output.len() {
            output[start..start + 4].copy_from_slice(&adjustment.to_be_bytes());
        }
    }

    Ok(output)
}

struct SfntTableRecord {
    tag: [u8; 4],
    checksum: u32,
    offset: usize,
    length: usize,
}

fn sfnt_search_params(num_tables: u16) -> (u16, u16, u16) {
    let max_power = 1u16 << (15 - num_tables.leading_zeros() as u16);
    let search_range = max_power * 16;
    let entry_selector = max_power.trailing_zeros() as u16;
    let range_shift = num_tables * 16 - search_range;
    (search_range, entry_selector, range_shift)
}

fn checksum(data: &[u8]) -> u32 {
    let mut sum = 0u32;
    for chunk in data.chunks(4) {
        let mut padded = [0u8; 4];
        padded[..chunk.len()].copy_from_slice(chunk);
        sum = sum.wrapping_add(u32::from_be_bytes(padded));
    }
    sum
}

fn align4(value: usize) -> usize {
    (value + 3) & !3
}
