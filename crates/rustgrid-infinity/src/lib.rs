pub mod client;
pub mod connection;
pub mod decode;
pub mod driver;
pub mod expr;
pub mod protocol;
pub mod types;

pub use client::InfinityClient;
pub use connection::InfinityConnection;
pub use driver::InfinityDriver;

#[cfg(test)]
mod tests {
    use super::*;
    use rustgrid_core::{CellValue, Driver, FilterCondition, FilterNode, FilterOperator};
    use types::{ColumnField, ColumnType};

    #[test]
    fn test_driver_descriptor() {
        let driver = InfinityDriver::new();
        assert_eq!(driver.id().as_str(), "infinity");
        assert_eq!(driver.display_name(), "Infinity");
        assert_eq!(driver.default_port(), 23817);

        let desc = driver.descriptor();
        assert_eq!(desc.id.as_str(), "infinity");
        assert_eq!(desc.icon, "icons/infinity.svg");
    }

    #[test]
    fn test_f16_and_bf16_conversion() {
        use decode::{bf16_to_f32, f16_to_f32};

        // 1.0 in f16: 0x3c00
        let f16_one = 0x3c00u16;
        let v = f16_to_f32(f16_one);
        assert!((v - 1.0).abs() < 1e-5);

        // 0.0 in f16
        assert_eq!(f16_to_f32(0), 0.0);

        // 1.0 in bf16: 0x3f80
        let bf16_one = 0x3f80u16;
        let v_bf = bf16_to_f32(bf16_one);
        assert!((v_bf - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_date_and_time_decoding() {
        use decode::{decode_date, decode_datetime, decode_time};

        // 0 days since 1970-01-01
        assert_eq!(decode_date(0), "1970-01-01");
        // 19723 days = 2024-01-01
        assert_eq!(decode_date(19723), "2024-01-01");

        // 3661 seconds = 01:01:01
        assert_eq!(decode_time(3661), "01:01:01");

        // DateTime
        assert_eq!(decode_datetime(0, 3600), "1970-01-01 01:00:00");
    }

    #[test]
    fn test_column_field_decoding_integers() {
        use decode::decode_column_field;

        // Two i32s: 42 and -100
        let mut buf = Vec::new();
        buf.extend_from_slice(&42i32.to_le_bytes());
        buf.extend_from_slice(&(-100i32).to_le_bytes());

        let field = ColumnField {
            column_type: ColumnType::ColumnInt32,
            column_vectors: vec![buf],
            column_name: "test_int".to_string(),
            bitmasks: vec![true, true],
        };

        let (info, rows) = decode_column_field(&field, None);
        assert_eq!(info.name, "test_int");
        assert_eq!(rows.len(), 2);
        assert!(matches!(rows[0], CellValue::Int(42)));
        assert!(matches!(rows[1], CellValue::Int(-100)));
    }

    #[test]
    fn test_column_field_decoding_null_bitmask() {
        use decode::decode_column_field;

        // Two i32s, but second row is NULL (bitmask = false)
        let mut buf = Vec::new();
        buf.extend_from_slice(&100i32.to_le_bytes());
        buf.extend_from_slice(&0i32.to_le_bytes());

        let field = ColumnField {
            column_type: ColumnType::ColumnInt32,
            column_vectors: vec![buf],
            column_name: "test_null".to_string(),
            bitmasks: vec![true, false],
        };

        let (_info, rows) = decode_column_field(&field, None);
        assert_eq!(rows.len(), 2);
        assert!(matches!(rows[0], CellValue::Int(100)));
        assert!(matches!(rows[1], CellValue::Null));
    }

    #[test]
    fn test_column_field_decoding_varchar() {
        use decode::decode_column_field;

        // Varchar: length (u32 LE) + UTF-8 bytes
        let s1 = "hello";
        let s2 = "infinity";
        let mut buf = Vec::new();
        buf.extend_from_slice(&(s1.len() as u32).to_le_bytes());
        buf.extend_from_slice(s1.as_bytes());
        buf.extend_from_slice(&(s2.len() as u32).to_le_bytes());
        buf.extend_from_slice(s2.as_bytes());

        let field = ColumnField {
            column_type: ColumnType::ColumnVarchar,
            column_vectors: vec![buf],
            column_name: "text_col".to_string(),
            bitmasks: vec![true, true],
        };

        let (_, rows) = decode_column_field(&field, None);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].as_display(), "hello");
        assert_eq!(rows[1].as_display(), "infinity");
    }

    #[test]
    fn test_filter_parsing() {
        use expr::parse_filter;

        let node = FilterNode::Condition(FilterCondition {
            column: "age".to_string(),
            operator: FilterOperator::GreaterThan,
            value: "18".to_string(),
            value2: String::new(),
            conjunction: rustgrid_core::FilterConjunction::And,
            enabled: true,
        });

        let expr = parse_filter(&[node]).expect("parsed filter");
        assert!(expr.function_expr.is_some());
        let func = expr.function_expr.unwrap();
        assert_eq!(func.function_name, ">");
        assert_eq!(func.arguments.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_sql_parsing() {
        use expr::{ParsedSqlCommand, parse_sql};

        let res = parse_sql(
            "SELECT id, name FROM users WHERE age > 18 ORDER BY id DESC LIMIT 10 OFFSET 20",
        )
        .unwrap();
        match res {
            ParsedSqlCommand::Select {
                table_name,
                select_list,
                where_expr,
                order_by_list,
                limit_expr,
                offset_expr,
                ..
            } => {
                assert_eq!(table_name, "users");
                assert_eq!(select_list.len(), 2);
                assert!(where_expr.is_some());
                assert!(order_by_list.is_some());
                assert!(limit_expr.is_some());
                assert!(offset_expr.is_some());
            }
            _ => panic!("Expected Select"),
        }
    }

    #[tokio::test]
    async fn test_thrift_protocol_mock_roundtrip() {
        use crate::protocol::*;
        use tokio::io::duplex;

        let (client_io, mut server_io) = duplex(4096);
        let (read_half, write_half) = tokio::io::split(client_io);

        let mut client_reader = ThriftReader::new(read_half);
        let mut client_writer = ThriftWriter::new(write_half);

        tokio::spawn(async move {
            let mut server_reader = ThriftReader::new(&mut server_io);
            let (name, msg_type, seq_id) = server_reader.read_message_begin().await.unwrap();
            assert_eq!(name, "Ping");
            assert_eq!(msg_type, MSG_CALL);
            assert_eq!(seq_id, 100);

            // Read empty struct
            server_reader.read_struct_begin().await.unwrap();
            let (f_type, _) = server_reader.read_field_begin().await.unwrap();
            assert_eq!(f_type, ttype::STOP);
            server_reader.read_struct_end().await.unwrap();
            server_reader.read_message_end().await.unwrap();

            // Reply
            let mut server_writer = ThriftWriter::new(&mut server_io);
            server_writer
                .write_message_begin("Ping", MSG_REPLY, seq_id)
                .await
                .unwrap();
            server_writer
                .write_struct_begin("Ping_result")
                .await
                .unwrap();
            server_writer
                .write_field_begin("success", ttype::STRING, 0)
                .await
                .unwrap();
            server_writer.write_string("pong").await.unwrap();
            server_writer.write_field_end().await.unwrap();
            server_writer.write_field_stop().await.unwrap();
            server_writer.write_struct_end().await.unwrap();
            server_writer.write_message_end().await.unwrap();
            server_writer.flush().await.unwrap();
        });

        // Client call
        client_writer
            .write_message_begin("Ping", MSG_CALL, 100)
            .await
            .unwrap();
        client_writer.write_struct_begin("Ping_args").await.unwrap();
        client_writer.write_field_stop().await.unwrap();
        client_writer.write_struct_end().await.unwrap();
        client_writer.write_message_end().await.unwrap();
        client_writer.flush().await.unwrap();

        // Client read reply
        let (name, msg_type, seq_id) = client_reader.read_message_begin().await.unwrap();
        assert_eq!(name, "Ping");
        assert_eq!(msg_type, MSG_REPLY);
        assert_eq!(seq_id, 100);

        client_reader.read_struct_begin().await.unwrap();
        let (f_type, f_id) = client_reader.read_field_begin().await.unwrap();
        assert_eq!(f_type, ttype::STRING);
        assert_eq!(f_id, 0);
        let resp_str = client_reader.read_string().await.unwrap();
        assert_eq!(resp_str, "pong");
        client_reader.read_field_end().await.unwrap();

        let (f_type, _) = client_reader.read_field_begin().await.unwrap();
        assert_eq!(f_type, ttype::STOP);
        client_reader.read_struct_end().await.unwrap();
        client_reader.read_message_end().await.unwrap();
    }
}
