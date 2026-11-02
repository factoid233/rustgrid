use std::io;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub const VERSION_MASK: i32 = -65536; // 0xffff0000 as i32
pub const VERSION_1: i32 = -2147418112; // 0x80010000 as i32
pub const TYPE_MASK: i32 = 0x000000ff;

pub const MSG_CALL: i8 = 1;
pub const MSG_REPLY: i8 = 2;
pub const MSG_EXCEPTION: i8 = 3;
pub const MSG_ONEWAY: i8 = 4;

pub mod ttype {
    pub const STOP: u8 = 0;
    pub const VOID: u8 = 1;
    pub const BOOL: u8 = 2;
    pub const BYTE: u8 = 3;
    pub const DOUBLE: u8 = 4;
    pub const I16: u8 = 6;
    pub const I32: u8 = 8;
    pub const I64: u8 = 10;
    pub const STRING: u8 = 11;
    pub const STRUCT: u8 = 12;
    pub const MAP: u8 = 13;
    pub const SET: u8 = 14;
    pub const LIST: u8 = 15;
}

pub struct ThriftWriter<W> {
    writer: W,
}

impl<W: AsyncWrite + Unpin> ThriftWriter<W> {
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    pub fn get_mut(&mut self) -> &mut W {
        &mut self.writer
    }

    pub async fn flush(&mut self) -> io::Result<()> {
        self.writer.flush().await
    }

    pub async fn write_message_begin(
        &mut self,
        name: &str,
        msg_type: i8,
        seq_id: i32,
    ) -> io::Result<()> {
        let version = VERSION_1 | (msg_type as i32);
        self.write_i32(version).await?;
        self.write_string(name).await?;
        self.write_i32(seq_id).await
    }

    pub async fn write_message_end(&mut self) -> io::Result<()> {
        Ok(())
    }

    pub async fn write_struct_begin(&mut self, _name: &str) -> io::Result<()> {
        Ok(())
    }

    pub async fn write_struct_end(&mut self) -> io::Result<()> {
        Ok(())
    }

    pub async fn write_field_begin(
        &mut self,
        _name: &str,
        field_type: u8,
        field_id: i16,
    ) -> io::Result<()> {
        self.write_byte(field_type as i8).await?;
        self.write_i16(field_id).await
    }

    pub async fn write_field_end(&mut self) -> io::Result<()> {
        Ok(())
    }

    pub async fn write_field_stop(&mut self) -> io::Result<()> {
        self.write_byte(ttype::STOP as i8).await
    }

    pub async fn write_map_begin(
        &mut self,
        key_type: u8,
        val_type: u8,
        size: usize,
    ) -> io::Result<()> {
        self.write_byte(key_type as i8).await?;
        self.write_byte(val_type as i8).await?;
        self.write_i32(size as i32).await
    }

    pub async fn write_map_end(&mut self) -> io::Result<()> {
        Ok(())
    }

    pub async fn write_list_begin(&mut self, elem_type: u8, size: usize) -> io::Result<()> {
        self.write_byte(elem_type as i8).await?;
        self.write_i32(size as i32).await
    }

    pub async fn write_list_end(&mut self) -> io::Result<()> {
        Ok(())
    }

    pub async fn write_set_begin(&mut self, elem_type: u8, size: usize) -> io::Result<()> {
        self.write_byte(elem_type as i8).await?;
        self.write_i32(size as i32).await
    }

    pub async fn write_set_end(&mut self) -> io::Result<()> {
        Ok(())
    }

    pub async fn write_bool(&mut self, b: bool) -> io::Result<()> {
        self.write_byte(if b { 1 } else { 0 }).await
    }

    pub async fn write_byte(&mut self, b: i8) -> io::Result<()> {
        self.writer.write_u8(b as u8).await
    }

    pub async fn write_i16(&mut self, i: i16) -> io::Result<()> {
        self.writer.write_i16(i).await
    }

    pub async fn write_i32(&mut self, i: i32) -> io::Result<()> {
        self.writer.write_i32(i).await
    }

    pub async fn write_i64(&mut self, i: i64) -> io::Result<()> {
        self.writer.write_i64(i).await
    }

    pub async fn write_double(&mut self, d: f64) -> io::Result<()> {
        self.writer.write_f64(d).await
    }

    pub async fn write_binary(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.write_i32(bytes.len() as i32).await?;
        self.writer.write_all(bytes).await
    }

    pub async fn write_string(&mut self, s: &str) -> io::Result<()> {
        self.write_binary(s.as_bytes()).await
    }
}

pub struct ThriftReader<R> {
    reader: R,
}

impl<R: AsyncRead + Unpin> ThriftReader<R> {
    pub fn new(reader: R) -> Self {
        Self { reader }
    }

    pub fn get_mut(&mut self) -> &mut R {
        &mut self.reader
    }

    pub async fn read_message_begin(&mut self) -> io::Result<(String, i8, i32)> {
        let size = self.read_i32().await?;
        if size < 0 {
            let version = size & VERSION_MASK;
            if version != VERSION_1 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Bad Thrift version: {version:#x}"),
                ));
            }
            let msg_type = (size & TYPE_MASK) as i8;
            let name = self.read_string().await?;
            let seq_id = self.read_i32().await?;
            Ok((name, msg_type, seq_id))
        } else {
            // Old Thrift unframed without version
            let mut name_buf = vec![0u8; size as usize];
            self.reader.read_exact(&mut name_buf).await?;
            let name = String::from_utf8(name_buf)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            let msg_type = self.read_byte().await?;
            let seq_id = self.read_i32().await?;
            Ok((name, msg_type, seq_id))
        }
    }

    pub async fn read_message_end(&mut self) -> io::Result<()> {
        Ok(())
    }

    pub async fn read_struct_begin(&mut self) -> io::Result<()> {
        Ok(())
    }

    pub async fn read_struct_end(&mut self) -> io::Result<()> {
        Ok(())
    }

    pub async fn read_field_begin(&mut self) -> io::Result<(u8, i16)> {
        let field_type = self.read_byte().await? as u8;
        let field_id = if field_type == ttype::STOP {
            0
        } else {
            self.read_i16().await?
        };
        Ok((field_type, field_id))
    }

    pub async fn read_field_end(&mut self) -> io::Result<()> {
        Ok(())
    }

    pub async fn read_map_begin(&mut self) -> io::Result<(u8, u8, usize)> {
        let key_type = self.read_byte().await? as u8;
        let val_type = self.read_byte().await? as u8;
        let size = self.read_i32().await?;
        if size < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Negative map size",
            ));
        }
        Ok((key_type, val_type, size as usize))
    }

    pub async fn read_map_end(&mut self) -> io::Result<()> {
        Ok(())
    }

    pub async fn read_list_begin(&mut self) -> io::Result<(u8, usize)> {
        let elem_type = self.read_byte().await? as u8;
        let size = self.read_i32().await?;
        if size < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Negative list size",
            ));
        }
        Ok((elem_type, size as usize))
    }

    pub async fn read_list_end(&mut self) -> io::Result<()> {
        Ok(())
    }

    pub async fn read_set_begin(&mut self) -> io::Result<(u8, usize)> {
        let elem_type = self.read_byte().await? as u8;
        let size = self.read_i32().await?;
        if size < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Negative set size",
            ));
        }
        Ok((elem_type, size as usize))
    }

    pub async fn read_set_end(&mut self) -> io::Result<()> {
        Ok(())
    }

    pub async fn read_bool(&mut self) -> io::Result<bool> {
        let b = self.read_byte().await?;
        Ok(b != 0)
    }

    pub async fn read_byte(&mut self) -> io::Result<i8> {
        let b = self.reader.read_u8().await?;
        Ok(b as i8)
    }

    pub async fn read_i16(&mut self) -> io::Result<i16> {
        self.reader.read_i16().await
    }

    pub async fn read_i32(&mut self) -> io::Result<i32> {
        self.reader.read_i32().await
    }

    pub async fn read_i64(&mut self) -> io::Result<i64> {
        self.reader.read_i64().await
    }

    pub async fn read_double(&mut self) -> io::Result<f64> {
        self.reader.read_f64().await
    }

    pub async fn read_binary(&mut self) -> io::Result<Vec<u8>> {
        let size = self.read_i32().await?;
        if size < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Negative binary length",
            ));
        }
        let mut buf = vec![0u8; size as usize];
        self.reader.read_exact(&mut buf).await?;
        Ok(buf)
    }

    pub async fn read_string(&mut self) -> io::Result<String> {
        let bytes = self.read_binary().await?;
        String::from_utf8(bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    pub async fn skip(&mut self, field_type: u8, depth: usize) -> io::Result<()> {
        if depth > 64 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Thrift skip depth limit reached",
            ));
        }
        match field_type {
            ttype::BOOL => {
                let _ = self.read_bool().await?;
            }
            ttype::BYTE => {
                let _ = self.read_byte().await?;
            }
            ttype::I16 => {
                let _ = self.read_i16().await?;
            }
            ttype::I32 => {
                let _ = self.read_i32().await?;
            }
            ttype::I64 => {
                let _ = self.read_i64().await?;
            }
            ttype::DOUBLE => {
                let _ = self.read_double().await?;
            }
            ttype::STRING => {
                let _ = self.read_binary().await?;
            }
            ttype::STRUCT => {
                self.read_struct_begin().await?;
                loop {
                    let (f_type, _) = self.read_field_begin().await?;
                    if f_type == ttype::STOP {
                        break;
                    }
                    Box::pin(self.skip(f_type, depth + 1)).await?;
                    self.read_field_end().await?;
                }
                self.read_struct_end().await?;
            }
            ttype::MAP => {
                let (k_type, v_type, size) = self.read_map_begin().await?;
                for _ in 0..size {
                    Box::pin(self.skip(k_type, depth + 1)).await?;
                    Box::pin(self.skip(v_type, depth + 1)).await?;
                }
                self.read_map_end().await?;
            }
            ttype::SET => {
                let (elem_type, size) = self.read_set_begin().await?;
                for _ in 0..size {
                    Box::pin(self.skip(elem_type, depth + 1)).await?;
                }
                self.read_set_end().await?;
            }
            ttype::LIST => {
                let (elem_type, size) = self.read_list_begin().await?;
                for _ in 0..size {
                    Box::pin(self.skip(elem_type, depth + 1)).await?;
                }
                self.read_list_end().await?;
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Cannot skip unknown Thrift type: {field_type}"),
                ));
            }
        }
        Ok(())
    }
}
