use chrono::{Datelike, NaiveDate};
use rustgrid_core::{CellValue, ColumnInfo};

use crate::types::{ColumnField, ColumnType};

pub fn f16_to_f32(bits: u16) -> f32 {
    let sign = ((bits >> 15) & 1) as u32;
    let exp = ((bits >> 10) & 0x1f) as u32;
    let mant = (bits & 0x3ff) as u32;

    if exp == 0 {
        if mant == 0 {
            f32::from_bits(sign << 31)
        } else {
            // subnormal
            let mut m = mant;
            let mut e = 0;
            while (m & 0x400) == 0 {
                m <<= 1;
                e += 1;
            }
            let exp_f32 = 127 - 15 - e + 1;
            let mant_f32 = (m & 0x3ff) << 13;
            f32::from_bits((sign << 31) | (exp_f32 << 23) | mant_f32)
        }
    } else if exp == 31 {
        if mant == 0 {
            // inf
            f32::from_bits((sign << 31) | 0x7f800000)
        } else {
            // nan
            f32::from_bits((sign << 31) | 0x7f800000 | (mant << 13))
        }
    } else {
        // normalized
        let exp_f32 = exp + 127 - 15;
        let mant_f32 = mant << 13;
        f32::from_bits((sign << 31) | (exp_f32 << 23) | mant_f32)
    }
}

pub fn bf16_to_f32(bits: u16) -> f32 {
    f32::from_bits((bits as u32) << 16)
}

pub fn decode_date(days: i32) -> String {
    // days since 1970-01-01
    let epoch = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
    match epoch.checked_add_signed(chrono::Duration::days(days as i64)) {
        Some(d) => d.format("%Y-%m-%d").to_string(),
        None => format!("{days} days"),
    }
}

pub fn decode_time(seconds: i32) -> String {
    let s = seconds.rem_euclid(86400);
    let h = s / 3600;
    let m = (s % 3600) / 60;
    let sec = s % 60;
    format!("{h:02}:{m:02}:{sec:02}")
}

pub fn decode_datetime(days: i32, seconds: i32) -> String {
    let epoch = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
    let d = epoch
        .checked_add_signed(chrono::Duration::days(days as i64))
        .unwrap_or(epoch);
    let s = seconds.rem_euclid(86400);
    let h = s / 3600;
    let m = (s % 3600) / 60;
    let sec = s % 60;
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        d.year(),
        d.month(),
        d.day(),
        h,
        m,
        sec
    )
}

pub fn decode_column_field(
    field: &ColumnField,
    expected_row_count: Option<usize>,
) -> (ColumnInfo, Vec<CellValue>) {
    let column_info = ColumnInfo {
        name: field.column_name.clone(),
        data_type: field.column_type.display_name().to_string(),
        nullable: true,
        primary_key: field.column_name == "_row_id",
        comment: String::new(),
    };

    let buf: Vec<u8> = field.column_vectors.concat();
    let num_rows = if !field.bitmasks.is_empty() {
        field.bitmasks.len()
    } else if let Some(n) = expected_row_count {
        n
    } else {
        match field.column_type {
            ColumnType::ColumnBool | ColumnType::ColumnInt8 => buf.len(),
            ColumnType::ColumnInt16 | ColumnType::ColumnFloat16 | ColumnType::ColumnBFloat16 => {
                buf.len() / 2
            }
            ColumnType::ColumnInt32
            | ColumnType::ColumnFloat32
            | ColumnType::ColumnDate
            | ColumnType::ColumnTime
            | ColumnType::ColumnInterval => buf.len() / 4,
            ColumnType::ColumnInt64
            | ColumnType::ColumnFloat64
            | ColumnType::ColumnRowID
            | ColumnType::ColumnDateTime
            | ColumnType::ColumnTimestamp => buf.len() / 8,
            ColumnType::ColumnVarchar | ColumnType::ColumnJson => {
                let mut c = 0;
                let mut count = 0;
                while c + 4 <= buf.len() {
                    let len =
                        u32::from_le_bytes([buf[c], buf[c + 1], buf[c + 2], buf[c + 3]]) as usize;
                    c += 4 + len;
                    count += 1;
                }
                count
            }
            _ => 0,
        }
    };

    let mut rows = Vec::with_capacity(num_rows);
    let mut cursor = 0;

    for row_idx in 0..num_rows {
        let is_valid = field.bitmasks.get(row_idx).copied().unwrap_or(true);
        if !is_valid {
            // Null row
            // Advance cursor if fixed-width
            match field.column_type {
                ColumnType::ColumnBool | ColumnType::ColumnInt8 => cursor += 1,
                ColumnType::ColumnInt16
                | ColumnType::ColumnFloat16
                | ColumnType::ColumnBFloat16 => cursor += 2,
                ColumnType::ColumnInt32
                | ColumnType::ColumnFloat32
                | ColumnType::ColumnDate
                | ColumnType::ColumnTime
                | ColumnType::ColumnInterval => cursor += 4,
                ColumnType::ColumnInt64
                | ColumnType::ColumnFloat64
                | ColumnType::ColumnRowID
                | ColumnType::ColumnDateTime
                | ColumnType::ColumnTimestamp => cursor += 8,
                ColumnType::ColumnVarchar | ColumnType::ColumnJson if cursor + 4 <= buf.len() => {
                    let len = u32::from_le_bytes([
                        buf[cursor],
                        buf[cursor + 1],
                        buf[cursor + 2],
                        buf[cursor + 3],
                    ]) as usize;
                    cursor += 4 + len;
                }
                _ => {}
            }
            rows.push(CellValue::Null);
            continue;
        }

        // Decode non-null value
        let val = match field.column_type {
            ColumnType::ColumnBool => {
                if cursor < buf.len() {
                    let b = buf[cursor] != 0;
                    cursor += 1;
                    CellValue::Bool(b)
                } else {
                    CellValue::Null
                }
            }
            ColumnType::ColumnInt8 => {
                if cursor < buf.len() {
                    let v = buf[cursor] as i8;
                    cursor += 1;
                    CellValue::Int(v as i64)
                } else {
                    CellValue::Null
                }
            }
            ColumnType::ColumnInt16 => {
                if cursor + 2 <= buf.len() {
                    let v = i16::from_le_bytes([buf[cursor], buf[cursor + 1]]);
                    cursor += 2;
                    CellValue::Int(v as i64)
                } else {
                    CellValue::Null
                }
            }
            ColumnType::ColumnInt32 => {
                if cursor + 4 <= buf.len() {
                    let v = i32::from_le_bytes([
                        buf[cursor],
                        buf[cursor + 1],
                        buf[cursor + 2],
                        buf[cursor + 3],
                    ]);
                    cursor += 4;
                    CellValue::Int(v as i64)
                } else {
                    CellValue::Null
                }
            }
            ColumnType::ColumnInt64 | ColumnType::ColumnRowID => {
                if cursor + 8 <= buf.len() {
                    let v = i64::from_le_bytes([
                        buf[cursor],
                        buf[cursor + 1],
                        buf[cursor + 2],
                        buf[cursor + 3],
                        buf[cursor + 4],
                        buf[cursor + 5],
                        buf[cursor + 6],
                        buf[cursor + 7],
                    ]);
                    cursor += 8;
                    CellValue::Int(v)
                } else {
                    CellValue::Null
                }
            }
            ColumnType::ColumnFloat32 => {
                if cursor + 4 <= buf.len() {
                    let v = f32::from_le_bytes([
                        buf[cursor],
                        buf[cursor + 1],
                        buf[cursor + 2],
                        buf[cursor + 3],
                    ]);
                    cursor += 4;
                    CellValue::Float(v as f64)
                } else {
                    CellValue::Null
                }
            }
            ColumnType::ColumnFloat64 => {
                if cursor + 8 <= buf.len() {
                    let v = f64::from_le_bytes([
                        buf[cursor],
                        buf[cursor + 1],
                        buf[cursor + 2],
                        buf[cursor + 3],
                        buf[cursor + 4],
                        buf[cursor + 5],
                        buf[cursor + 6],
                        buf[cursor + 7],
                    ]);
                    cursor += 8;
                    CellValue::Float(v)
                } else {
                    CellValue::Null
                }
            }
            ColumnType::ColumnFloat16 => {
                if cursor + 2 <= buf.len() {
                    let bits = u16::from_le_bytes([buf[cursor], buf[cursor + 1]]);
                    cursor += 2;
                    CellValue::Float(f16_to_f32(bits) as f64)
                } else {
                    CellValue::Null
                }
            }
            ColumnType::ColumnBFloat16 => {
                if cursor + 2 <= buf.len() {
                    let bits = u16::from_le_bytes([buf[cursor], buf[cursor + 1]]);
                    cursor += 2;
                    CellValue::Float(bf16_to_f32(bits) as f64)
                } else {
                    CellValue::Null
                }
            }
            ColumnType::ColumnDate => {
                if cursor + 4 <= buf.len() {
                    let days = i32::from_le_bytes([
                        buf[cursor],
                        buf[cursor + 1],
                        buf[cursor + 2],
                        buf[cursor + 3],
                    ]);
                    cursor += 4;
                    CellValue::Text(decode_date(days))
                } else {
                    CellValue::Null
                }
            }
            ColumnType::ColumnTime => {
                if cursor + 4 <= buf.len() {
                    let sec = i32::from_le_bytes([
                        buf[cursor],
                        buf[cursor + 1],
                        buf[cursor + 2],
                        buf[cursor + 3],
                    ]);
                    cursor += 4;
                    CellValue::Text(decode_time(sec))
                } else {
                    CellValue::Null
                }
            }
            ColumnType::ColumnDateTime | ColumnType::ColumnTimestamp => {
                if cursor + 8 <= buf.len() {
                    let days = i32::from_le_bytes([
                        buf[cursor],
                        buf[cursor + 1],
                        buf[cursor + 2],
                        buf[cursor + 3],
                    ]);
                    let seconds = i32::from_le_bytes([
                        buf[cursor + 4],
                        buf[cursor + 5],
                        buf[cursor + 6],
                        buf[cursor + 7],
                    ]);
                    cursor += 8;
                    CellValue::Text(decode_datetime(days, seconds))
                } else {
                    CellValue::Null
                }
            }
            ColumnType::ColumnInterval => {
                if cursor + 4 <= buf.len() {
                    let sec = i32::from_le_bytes([
                        buf[cursor],
                        buf[cursor + 1],
                        buf[cursor + 2],
                        buf[cursor + 3],
                    ]);
                    cursor += 4;
                    CellValue::Text(format!("{sec}s"))
                } else {
                    CellValue::Null
                }
            }
            ColumnType::ColumnVarchar | ColumnType::ColumnJson => {
                if cursor + 4 <= buf.len() {
                    let len = u32::from_le_bytes([
                        buf[cursor],
                        buf[cursor + 1],
                        buf[cursor + 2],
                        buf[cursor + 3],
                    ]) as usize;
                    cursor += 4;
                    let end = (cursor + len).min(buf.len());
                    let mut slice = &buf[cursor..end];
                    cursor = end;
                    // Trim trailing null bytes if present
                    if let Some(pos) = slice.iter().position(|&b| b == 0) {
                        slice = &slice[..pos];
                    }
                    let s = String::from_utf8_lossy(slice).to_string();
                    CellValue::Text(s)
                } else {
                    CellValue::Null
                }
            }
            ColumnType::ColumnEmbedding => {
                // If embedding, each row is embedding_size = buf.len() / num_rows
                let row_bytes = buf.len().checked_div(num_rows).unwrap_or(0);
                if row_bytes > 0 && cursor + row_bytes <= buf.len() {
                    let slice = &buf[cursor..cursor + row_bytes];
                    cursor += row_bytes;
                    // Attempt decode as float32 vector
                    let mut floats = Vec::with_capacity(slice.len() / 4);
                    for chunk in slice.as_chunks::<4>().0 {
                        let f = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                        floats.push(format!("{f:.4}"));
                    }
                    CellValue::Text(format!("[{}]", floats.join(", ")))
                } else {
                    CellValue::Null
                }
            }
            ColumnType::ColumnMultiVector | ColumnType::ColumnTensor => {
                if cursor + 4 <= buf.len() {
                    let len = u32::from_le_bytes([
                        buf[cursor],
                        buf[cursor + 1],
                        buf[cursor + 2],
                        buf[cursor + 3],
                    ]) as usize;
                    cursor += 4;
                    let end = (cursor + len).min(buf.len());
                    let slice = &buf[cursor..end];
                    cursor = end;
                    let mut floats = Vec::with_capacity(slice.len() / 4);
                    for chunk in slice.as_chunks::<4>().0 {
                        let f = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                        floats.push(format!("{f:.4}"));
                    }
                    CellValue::Text(format!("[{}]", floats.join(", ")))
                } else {
                    CellValue::Null
                }
            }
            ColumnType::ColumnSparse => {
                if cursor + 4 <= buf.len() {
                    let nnz = u32::from_le_bytes([
                        buf[cursor],
                        buf[cursor + 1],
                        buf[cursor + 2],
                        buf[cursor + 3],
                    ]) as usize;
                    cursor += 4;
                    // indices (i32) + values (f32)
                    let indices_bytes = nnz * 4;
                    let values_bytes = nnz * 4;
                    if cursor + indices_bytes + values_bytes <= buf.len() {
                        let mut pairs = Vec::with_capacity(nnz);
                        let idx_slice = &buf[cursor..cursor + indices_bytes];
                        cursor += indices_bytes;
                        let val_slice = &buf[cursor..cursor + values_bytes];
                        cursor += values_bytes;
                        for i in 0..nnz {
                            let idx = i32::from_le_bytes([
                                idx_slice[i * 4],
                                idx_slice[i * 4 + 1],
                                idx_slice[i * 4 + 2],
                                idx_slice[i * 4 + 3],
                            ]);
                            let val = f32::from_le_bytes([
                                val_slice[i * 4],
                                val_slice[i * 4 + 1],
                                val_slice[i * 4 + 2],
                                val_slice[i * 4 + 3],
                            ]);
                            pairs.push(format!("{idx}:{val:.4}"));
                        }
                        CellValue::Text(format!("{{{}}}", pairs.join(", ")))
                    } else {
                        CellValue::Null
                    }
                } else {
                    CellValue::Null
                }
            }
            ColumnType::ColumnArray => {
                if cursor + 4 <= buf.len() {
                    let elem_count = u32::from_le_bytes([
                        buf[cursor],
                        buf[cursor + 1],
                        buf[cursor + 2],
                        buf[cursor + 3],
                    ]) as usize;
                    cursor += 4;
                    CellValue::Text(format!("Array[{elem_count} elements]"))
                } else {
                    CellValue::Null
                }
            }
            _ => {
                if cursor < buf.len() {
                    cursor = buf.len();
                }
                CellValue::Null
            }
        };
        rows.push(val);
    }

    (column_info, rows)
}
