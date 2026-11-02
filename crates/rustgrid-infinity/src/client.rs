use std::io;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::protocol::{MSG_CALL, MSG_EXCEPTION, ThriftReader, ThriftWriter, ttype};
use crate::types::{
    ColumnDef, CommonResponse, ConnectResponse, CreateConflict, CreateOption, DropConflict,
    DropOption, ExplainType, Field, ListDatabaseResponse, ListTableResponse, OrderByExpr,
    ParsedExpr, SearchExpr, SelectResponse, UpdateExpr,
};

pub const DEFAULT_CLIENT_VERSION: i64 = 37;

pub type BoxedRead = Box<dyn AsyncRead + Send + Unpin>;
pub type BoxedWrite = Box<dyn AsyncWrite + Send + Unpin>;

pub struct InfinityClient {
    reader: ThriftReader<BoxedRead>,
    writer: ThriftWriter<BoxedWrite>,
    seq_id: i32,
    session_id: i64,
    server_version: String,
}

impl InfinityClient {
    pub async fn connect(
        read: BoxedRead,
        write: BoxedWrite,
        _user: Option<String>,
        _password: Option<String>,
    ) -> io::Result<Self> {
        let mut client = Self {
            reader: ThriftReader::new(read),
            writer: ThriftWriter::new(write),
            seq_id: 1,
            session_id: 0,
            server_version: String::new(),
        };

        // Try connect with latest client version (37)
        let mut resp = client.do_connect(DEFAULT_CLIENT_VERSION).await?;

        // If version mismatch (code 2003 or 2004), retry with expected version if detectable
        if (resp.error_code == 2003 || resp.error_code == 2004)
            && let Some(ver) = resp.error_msg.as_deref().and_then(extract_version_index)
        {
            tracing::info!(
                "Infinity version mismatch: retrying with client version {}",
                ver
            );
            resp = client.do_connect(ver).await?;
        }

        if !resp.is_ok() {
            return Err(io::Error::new(
                io::ErrorKind::ConnectionRefused,
                resp.error_message(),
            ));
        }

        client.session_id = resp.session_id;
        client.server_version = "0.7.4".to_string();
        Ok(client)
    }

    pub fn session_id(&self) -> i64 {
        self.session_id
    }

    pub fn server_version(&self) -> &str {
        &self.server_version
    }

    fn next_seq_id(&mut self) -> i32 {
        let id = self.seq_id;
        self.seq_id = self.seq_id.wrapping_add(1);
        id
    }

    async fn do_connect(&mut self, version: i64) -> io::Result<ConnectResponse> {
        let seq_id = self.next_seq_id();
        self.writer
            .write_message_begin("Connect", MSG_CALL, seq_id)
            .await?;

        // args struct
        self.writer.write_struct_begin("Connect_args").await?;

        // field 1: request
        self.writer
            .write_field_begin("request", ttype::STRUCT, 1)
            .await?;
        self.writer.write_struct_begin("ConnectRequest").await?;

        // 1: client_version (i64)
        self.writer
            .write_field_begin("client_version", ttype::I64, 1)
            .await?;
        self.writer.write_i64(version).await?;
        self.writer.write_field_end().await?;

        self.writer.write_field_stop().await?;
        self.writer.write_struct_end().await?;
        self.writer.write_field_end().await?;

        self.writer.write_field_stop().await?;
        self.writer.write_struct_end().await?;
        self.writer.write_message_end().await?;
        self.writer.flush().await?;

        self.read_common_response().await
    }

    pub async fn disconnect(&mut self) -> io::Result<()> {
        let seq_id = self.next_seq_id();
        self.writer
            .write_message_begin("Disconnect", MSG_CALL, seq_id)
            .await?;
        self.writer.write_struct_begin("Disconnect_args").await?;

        // 1: request (CommonRequest { session_id })
        self.writer
            .write_field_begin("request", ttype::STRUCT, 1)
            .await?;
        self.writer.write_struct_begin("CommonRequest").await?;
        self.writer
            .write_field_begin("session_id", ttype::I64, 1)
            .await?;
        self.writer.write_i64(self.session_id).await?;
        self.writer.write_field_end().await?;
        self.writer.write_field_stop().await?;
        self.writer.write_struct_end().await?;
        self.writer.write_field_end().await?;

        self.writer.write_field_stop().await?;
        self.writer.write_struct_end().await?;
        self.writer.write_message_end().await?;
        self.writer.flush().await?;

        let _ = self.read_common_response().await;
        Ok(())
    }

    pub async fn list_databases(&mut self) -> io::Result<Vec<String>> {
        let seq_id = self.next_seq_id();
        self.writer
            .write_message_begin("ListDatabase", MSG_CALL, seq_id)
            .await?;
        self.writer.write_struct_begin("ListDatabase_args").await?;

        self.writer
            .write_field_begin("request", ttype::STRUCT, 1)
            .await?;
        self.writer
            .write_struct_begin("ListDatabaseRequest")
            .await?;
        self.writer
            .write_field_begin("session_id", ttype::I64, 1)
            .await?;
        self.writer.write_i64(self.session_id).await?;
        self.writer.write_field_end().await?;
        self.writer.write_field_stop().await?;
        self.writer.write_struct_end().await?;
        self.writer.write_field_end().await?;

        self.writer.write_field_stop().await?;
        self.writer.write_struct_end().await?;
        self.writer.write_message_end().await?;
        self.writer.flush().await?;

        let (_, msg_type, _) = self.reader.read_message_begin().await?;
        if msg_type == MSG_EXCEPTION {
            return Err(io::Error::other(
                "Thrift application exception during ListDatabase",
            ));
        }

        let mut resp = ListDatabaseResponse::default();
        self.reader.read_struct_begin().await?;
        loop {
            let (f_type, f_id) = self.reader.read_field_begin().await?;
            if f_type == ttype::STOP {
                break;
            }
            if f_id == 0 {
                resp = ListDatabaseResponse::read(&mut self.reader).await?;
            } else {
                self.reader.skip(f_type, 0).await?;
            }
            self.reader.read_field_end().await?;
        }
        self.reader.read_struct_end().await?;
        self.reader.read_message_end().await?;

        if resp.error_code != 0 {
            return Err(io::Error::other(resp.error_msg.unwrap_or_else(|| {
                format!("ListDatabase failed: code {}", resp.error_code)
            })));
        }

        Ok(resp.db_names.unwrap_or_default())
    }

    pub async fn show_current_database(&mut self) -> io::Result<String> {
        Ok("default_db".to_string())
    }

    pub async fn create_database(
        &mut self,
        db_name: &str,
        comment: Option<&str>,
    ) -> io::Result<()> {
        let seq_id = self.next_seq_id();
        self.writer
            .write_message_begin("CreateDatabase", MSG_CALL, seq_id)
            .await?;
        self.writer
            .write_struct_begin("CreateDatabase_args")
            .await?;

        self.writer
            .write_field_begin("request", ttype::STRUCT, 1)
            .await?;
        self.writer
            .write_struct_begin("CreateDatabaseRequest")
            .await?;

        // 1: db_name
        self.writer
            .write_field_begin("db_name", ttype::STRING, 1)
            .await?;
        self.writer.write_string(db_name).await?;
        self.writer.write_field_end().await?;

        // 2: session_id
        self.writer
            .write_field_begin("session_id", ttype::I64, 2)
            .await?;
        self.writer.write_i64(self.session_id).await?;
        self.writer.write_field_end().await?;

        // 3: create_option
        self.writer
            .write_field_begin("create_option", ttype::STRUCT, 3)
            .await?;
        let opt = CreateOption::new(CreateConflict::Error);
        opt.write(&mut self.writer).await?;
        self.writer.write_field_end().await?;

        // 4: db_comment
        if let Some(c) = comment {
            self.writer
                .write_field_begin("db_comment", ttype::STRING, 4)
                .await?;
            self.writer.write_string(c).await?;
            self.writer.write_field_end().await?;
        }

        self.writer.write_field_stop().await?;
        self.writer.write_struct_end().await?;
        self.writer.write_field_end().await?;

        self.writer.write_field_stop().await?;
        self.writer.write_struct_end().await?;
        self.writer.write_message_end().await?;
        self.writer.flush().await?;

        let resp = self.read_common_response().await?;
        if !resp.is_ok() {
            return Err(io::Error::other(resp.error_message()));
        }
        Ok(())
    }

    pub async fn drop_database(&mut self, db_name: &str) -> io::Result<()> {
        let seq_id = self.next_seq_id();
        self.writer
            .write_message_begin("DropDatabase", MSG_CALL, seq_id)
            .await?;
        self.writer.write_struct_begin("DropDatabase_args").await?;

        self.writer
            .write_field_begin("request", ttype::STRUCT, 1)
            .await?;
        self.writer
            .write_struct_begin("DropDatabaseRequest")
            .await?;

        // 1: db_name
        self.writer
            .write_field_begin("db_name", ttype::STRING, 1)
            .await?;
        self.writer.write_string(db_name).await?;
        self.writer.write_field_end().await?;

        // 2: session_id
        self.writer
            .write_field_begin("session_id", ttype::I64, 2)
            .await?;
        self.writer.write_i64(self.session_id).await?;
        self.writer.write_field_end().await?;

        // 3: drop_option
        self.writer
            .write_field_begin("drop_option", ttype::STRUCT, 3)
            .await?;
        let opt = DropOption::new(DropConflict::Error);
        opt.write(&mut self.writer).await?;
        self.writer.write_field_end().await?;

        self.writer.write_field_stop().await?;
        self.writer.write_struct_end().await?;
        self.writer.write_field_end().await?;

        self.writer.write_field_stop().await?;
        self.writer.write_struct_end().await?;
        self.writer.write_message_end().await?;
        self.writer.flush().await?;

        let resp = self.read_common_response().await?;
        if !resp.is_ok() {
            return Err(io::Error::other(resp.error_message()));
        }
        Ok(())
    }

    pub async fn use_database(&mut self, db_name: &str) -> io::Result<()> {
        let seq_id = self.next_seq_id();
        self.writer
            .write_message_begin("GetDatabase", MSG_CALL, seq_id)
            .await?;
        self.writer.write_struct_begin("GetDatabase_args").await?;

        self.writer
            .write_field_begin("request", ttype::STRUCT, 1)
            .await?;
        self.writer.write_struct_begin("GetDatabaseRequest").await?;

        // 1: db_name
        self.writer
            .write_field_begin("db_name", ttype::STRING, 1)
            .await?;
        self.writer.write_string(db_name).await?;
        self.writer.write_field_end().await?;

        // 2: session_id
        self.writer
            .write_field_begin("session_id", ttype::I64, 2)
            .await?;
        self.writer.write_i64(self.session_id).await?;
        self.writer.write_field_end().await?;

        self.writer.write_field_stop().await?;
        self.writer.write_struct_end().await?;
        self.writer.write_field_end().await?;

        self.writer.write_field_stop().await?;
        self.writer.write_struct_end().await?;
        self.writer.write_message_end().await?;
        self.writer.flush().await?;

        let resp = self.read_common_response().await?;
        if !resp.is_ok() {
            return Err(io::Error::other(resp.error_message()));
        }
        Ok(())
    }

    pub async fn list_tables(&mut self, db_name: &str) -> io::Result<Vec<String>> {
        let seq_id = self.next_seq_id();
        self.writer
            .write_message_begin("ListTable", MSG_CALL, seq_id)
            .await?;
        self.writer.write_struct_begin("ListTable_args").await?;

        self.writer
            .write_field_begin("request", ttype::STRUCT, 1)
            .await?;
        self.writer.write_struct_begin("ListTableRequest").await?;

        // 1: db_name
        self.writer
            .write_field_begin("db_name", ttype::STRING, 1)
            .await?;
        self.writer.write_string(db_name).await?;
        self.writer.write_field_end().await?;

        // 2: session_id
        self.writer
            .write_field_begin("session_id", ttype::I64, 2)
            .await?;
        self.writer.write_i64(self.session_id).await?;
        self.writer.write_field_end().await?;

        self.writer.write_field_stop().await?;
        self.writer.write_struct_end().await?;
        self.writer.write_field_end().await?;

        self.writer.write_field_stop().await?;
        self.writer.write_struct_end().await?;
        self.writer.write_message_end().await?;
        self.writer.flush().await?;

        let (_, msg_type, _) = self.reader.read_message_begin().await?;
        if msg_type == MSG_EXCEPTION {
            return Err(io::Error::other(
                "Thrift application exception during ListTable",
            ));
        }

        let mut resp = ListTableResponse::default();
        self.reader.read_struct_begin().await?;
        loop {
            let (f_type, f_id) = self.reader.read_field_begin().await?;
            if f_type == ttype::STOP {
                break;
            }
            if f_id == 0 {
                resp = ListTableResponse::read(&mut self.reader).await?;
            } else {
                self.reader.skip(f_type, 0).await?;
            }
            self.reader.read_field_end().await?;
        }
        self.reader.read_struct_end().await?;
        self.reader.read_message_end().await?;

        if resp.error_code != 0 {
            return Err(io::Error::other(resp.error_msg.unwrap_or_else(|| {
                format!("ListTable failed: code {}", resp.error_code)
            })));
        }

        Ok(resp.table_names.unwrap_or_default())
    }

    pub async fn show_tables(&mut self, db_name: &str) -> io::Result<SelectResponse> {
        let tables = self.list_tables(db_name).await?;
        let col = ColumnDef {
            id: 1,
            data_type: crate::types::DataType::simple(crate::types::LogicType::Varchar),
            name: format!("Tables_in_{db_name}"),
            constraints: Vec::new(),
            constant_expr: None,
            comment: None,
        };
        let cf = crate::types::ColumnField {
            column_name: col.name.clone(),
            column_type: crate::types::ColumnType::ColumnVarchar,
            column_vectors: tables.iter().map(|t| t.as_bytes().to_vec()).collect(),
            bitmasks: vec![true; tables.len()],
        };
        Ok(SelectResponse {
            error_code: 0,
            error_msg: None,
            column_defs: Some(vec![col]),
            column_fields: Some(vec![cf]),
            extra_result: None,
        })
    }

    pub async fn show_columns(
        &mut self,
        db_name: &str,
        table_name: &str,
    ) -> io::Result<SelectResponse> {
        let seq_id = self.next_seq_id();
        self.writer
            .write_message_begin("ShowColumns", MSG_CALL, seq_id)
            .await?;
        self.writer.write_struct_begin("ShowColumns_args").await?;

        self.writer
            .write_field_begin("request", ttype::STRUCT, 1)
            .await?;
        self.writer.write_struct_begin("ShowColumnsRequest").await?;

        // 1: db_name
        self.writer
            .write_field_begin("db_name", ttype::STRING, 1)
            .await?;
        self.writer.write_string(db_name).await?;
        self.writer.write_field_end().await?;

        // 2: table_name
        self.writer
            .write_field_begin("table_name", ttype::STRING, 2)
            .await?;
        self.writer.write_string(table_name).await?;
        self.writer.write_field_end().await?;

        // 3: session_id
        self.writer
            .write_field_begin("session_id", ttype::I64, 3)
            .await?;
        self.writer.write_i64(self.session_id).await?;
        self.writer.write_field_end().await?;

        self.writer.write_field_stop().await?;
        self.writer.write_struct_end().await?;
        self.writer.write_field_end().await?;

        self.writer.write_field_stop().await?;
        self.writer.write_struct_end().await?;
        self.writer.write_message_end().await?;
        self.writer.flush().await?;

        self.read_select_response().await
    }

    pub async fn create_table(
        &mut self,
        db_name: &str,
        table_name: &str,
        columns: &[ColumnDef],
        _comment: Option<&str>,
    ) -> io::Result<()> {
        let seq_id = self.next_seq_id();
        self.writer
            .write_message_begin("CreateTable", MSG_CALL, seq_id)
            .await?;
        self.writer.write_struct_begin("CreateTable_args").await?;

        self.writer
            .write_field_begin("request", ttype::STRUCT, 1)
            .await?;
        self.writer.write_struct_begin("CreateTableRequest").await?;

        // 1: db_name
        self.writer
            .write_field_begin("db_name", ttype::STRING, 1)
            .await?;
        self.writer.write_string(db_name).await?;
        self.writer.write_field_end().await?;

        // 2: table_name
        self.writer
            .write_field_begin("table_name", ttype::STRING, 2)
            .await?;
        self.writer.write_string(table_name).await?;
        self.writer.write_field_end().await?;

        // 3: column_defs
        self.writer
            .write_field_begin("column_defs", ttype::LIST, 3)
            .await?;
        self.writer
            .write_list_begin(ttype::STRUCT, columns.len())
            .await?;
        for col in columns {
            col.write(&mut self.writer).await?;
        }
        self.writer.write_list_end().await?;
        self.writer.write_field_end().await?;

        // 6: session_id
        self.writer
            .write_field_begin("session_id", ttype::I64, 6)
            .await?;
        self.writer.write_i64(self.session_id).await?;
        self.writer.write_field_end().await?;

        // 7: create_option
        self.writer
            .write_field_begin("create_option", ttype::STRUCT, 7)
            .await?;
        let opt = CreateOption::new(CreateConflict::Error);
        opt.write(&mut self.writer).await?;
        self.writer.write_field_end().await?;

        self.writer.write_field_stop().await?;
        self.writer.write_struct_end().await?;
        self.writer.write_field_end().await?;

        self.writer.write_field_stop().await?;
        self.writer.write_struct_end().await?;
        self.writer.write_message_end().await?;
        self.writer.flush().await?;

        let resp = self.read_common_response().await?;
        if !resp.is_ok() {
            return Err(io::Error::other(resp.error_message()));
        }
        Ok(())
    }

    pub async fn drop_table(&mut self, db_name: &str, table_name: &str) -> io::Result<()> {
        let seq_id = self.next_seq_id();
        self.writer
            .write_message_begin("DropTable", MSG_CALL, seq_id)
            .await?;
        self.writer.write_struct_begin("DropTable_args").await?;

        self.writer
            .write_field_begin("request", ttype::STRUCT, 1)
            .await?;
        self.writer.write_struct_begin("DropTableRequest").await?;

        // 1: db_name
        self.writer
            .write_field_begin("db_name", ttype::STRING, 1)
            .await?;
        self.writer.write_string(db_name).await?;
        self.writer.write_field_end().await?;

        // 2: table_name
        self.writer
            .write_field_begin("table_name", ttype::STRING, 2)
            .await?;
        self.writer.write_string(table_name).await?;
        self.writer.write_field_end().await?;

        // 3: session_id
        self.writer
            .write_field_begin("session_id", ttype::I64, 3)
            .await?;
        self.writer.write_i64(self.session_id).await?;
        self.writer.write_field_end().await?;

        // 4: drop_option
        self.writer
            .write_field_begin("drop_option", ttype::STRUCT, 4)
            .await?;
        let opt = DropOption::new(DropConflict::Error);
        opt.write(&mut self.writer).await?;
        self.writer.write_field_end().await?;

        self.writer.write_field_stop().await?;
        self.writer.write_struct_end().await?;
        self.writer.write_field_end().await?;

        self.writer.write_field_stop().await?;
        self.writer.write_struct_end().await?;
        self.writer.write_message_end().await?;
        self.writer.flush().await?;

        let resp = self.read_common_response().await?;
        if !resp.is_ok() {
            return Err(io::Error::other(resp.error_message()));
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn select(
        &mut self,
        db_name: &str,
        table_name: &str,
        select_list: &[ParsedExpr],
        search_expr: Option<&SearchExpr>,
        where_expr: Option<&ParsedExpr>,
        group_by_list: Option<&[ParsedExpr]>,
        order_by_list: Option<&[OrderByExpr]>,
        limit_expr: Option<&ParsedExpr>,
        offset_expr: Option<&ParsedExpr>,
        total_hits_count: bool,
    ) -> io::Result<SelectResponse> {
        let seq_id = self.next_seq_id();
        self.writer
            .write_message_begin("Select", MSG_CALL, seq_id)
            .await?;
        self.writer.write_struct_begin("Select_args").await?;

        self.writer
            .write_field_begin("request", ttype::STRUCT, 1)
            .await?;
        self.writer.write_struct_begin("SelectRequest").await?;

        // 1: session_id
        self.writer
            .write_field_begin("session_id", ttype::I64, 1)
            .await?;
        self.writer.write_i64(self.session_id).await?;
        self.writer.write_field_end().await?;

        // 2: db_name
        self.writer
            .write_field_begin("db_name", ttype::STRING, 2)
            .await?;
        self.writer.write_string(db_name).await?;
        self.writer.write_field_end().await?;

        // 3: table_name
        self.writer
            .write_field_begin("table_name", ttype::STRING, 3)
            .await?;
        self.writer.write_string(table_name).await?;
        self.writer.write_field_end().await?;

        // 4: select_list
        self.writer
            .write_field_begin("select_list", ttype::LIST, 4)
            .await?;
        self.writer
            .write_list_begin(ttype::STRUCT, select_list.len())
            .await?;
        for expr in select_list {
            expr.write(&mut self.writer).await?;
        }
        self.writer.write_list_end().await?;
        self.writer.write_field_end().await?;

        // 6: search_expr
        if let Some(search) = search_expr {
            self.writer
                .write_field_begin("search_expr", ttype::STRUCT, 6)
                .await?;
            search.write(&mut self.writer).await?;
            self.writer.write_field_end().await?;
        }

        // 7: where_expr
        if let Some(w) = where_expr {
            self.writer
                .write_field_begin("where_expr", ttype::STRUCT, 7)
                .await?;
            w.write(&mut self.writer).await?;
            self.writer.write_field_end().await?;
        }

        // 8: group_by_list
        if let Some(groups) = group_by_list {
            self.writer
                .write_field_begin("group_by_list", ttype::LIST, 8)
                .await?;
            self.writer
                .write_list_begin(ttype::STRUCT, groups.len())
                .await?;
            for g in groups {
                g.write(&mut self.writer).await?;
            }
            self.writer.write_list_end().await?;
            self.writer.write_field_end().await?;
        }

        // 10: limit_expr
        if let Some(lim) = limit_expr {
            self.writer
                .write_field_begin("limit_expr", ttype::STRUCT, 10)
                .await?;
            lim.write(&mut self.writer).await?;
            self.writer.write_field_end().await?;
        }

        // 11: offset_expr
        if let Some(off) = offset_expr {
            self.writer
                .write_field_begin("offset_expr", ttype::STRUCT, 11)
                .await?;
            off.write(&mut self.writer).await?;
            self.writer.write_field_end().await?;
        }

        // 12: order_by_list
        if let Some(orders) = order_by_list {
            self.writer
                .write_field_begin("order_by_list", ttype::LIST, 12)
                .await?;
            self.writer
                .write_list_begin(ttype::STRUCT, orders.len())
                .await?;
            for o in orders {
                o.write(&mut self.writer).await?;
            }
            self.writer.write_list_end().await?;
            self.writer.write_field_end().await?;
        }

        // 13: total_hits_count (only valid when limit_expr is present)
        if total_hits_count && limit_expr.is_some() {
            self.writer
                .write_field_begin("total_hits_count", ttype::BOOL, 13)
                .await?;
            self.writer.write_bool(true).await?;
            self.writer.write_field_end().await?;
        }

        self.writer.write_field_stop().await?;
        self.writer.write_struct_end().await?;
        self.writer.write_field_end().await?;

        self.writer.write_field_stop().await?;
        self.writer.write_struct_end().await?;
        self.writer.write_message_end().await?;
        self.writer.flush().await?;

        self.read_select_response().await
    }

    pub async fn insert(
        &mut self,
        db_name: &str,
        table_name: &str,
        column_names: &[String],
        values: &[Vec<ParsedExpr>],
    ) -> io::Result<()> {
        let seq_id = self.next_seq_id();
        self.writer
            .write_message_begin("Insert", MSG_CALL, seq_id)
            .await?;
        self.writer.write_struct_begin("Insert_args").await?;

        self.writer
            .write_field_begin("request", ttype::STRUCT, 1)
            .await?;
        self.writer.write_struct_begin("InsertRequest").await?;

        // 1: db_name
        self.writer
            .write_field_begin("db_name", ttype::STRING, 1)
            .await?;
        self.writer.write_string(db_name).await?;
        self.writer.write_field_end().await?;

        // 2: table_name
        self.writer
            .write_field_begin("table_name", ttype::STRING, 2)
            .await?;
        self.writer.write_string(table_name).await?;
        self.writer.write_field_end().await?;

        // 3: fields (list<Field>)
        self.writer
            .write_field_begin("fields", ttype::LIST, 3)
            .await?;
        self.writer
            .write_list_begin(ttype::STRUCT, values.len())
            .await?;
        for row in values {
            let field = Field {
                column_names: column_names.to_vec(),
                parse_exprs: row.clone(),
            };
            field.write(&mut self.writer).await?;
        }
        self.writer.write_list_end().await?;
        self.writer.write_field_end().await?;

        // 4: session_id
        self.writer
            .write_field_begin("session_id", ttype::I64, 4)
            .await?;
        self.writer.write_i64(self.session_id).await?;
        self.writer.write_field_end().await?;

        self.writer.write_field_stop().await?;
        self.writer.write_struct_end().await?;
        self.writer.write_field_end().await?;

        self.writer.write_field_stop().await?;
        self.writer.write_struct_end().await?;
        self.writer.write_message_end().await?;
        self.writer.flush().await?;

        let resp = self.read_common_response().await?;
        if !resp.is_ok() {
            return Err(io::Error::other(resp.error_message()));
        }
        Ok(())
    }

    pub async fn update(
        &mut self,
        db_name: &str,
        table_name: &str,
        where_expr: Option<&ParsedExpr>,
        updates: &[UpdateExpr],
    ) -> io::Result<()> {
        let seq_id = self.next_seq_id();
        self.writer
            .write_message_begin("Update", MSG_CALL, seq_id)
            .await?;
        self.writer.write_struct_begin("Update_args").await?;

        self.writer
            .write_field_begin("request", ttype::STRUCT, 1)
            .await?;
        self.writer.write_struct_begin("UpdateRequest").await?;

        // 1: db_name
        self.writer
            .write_field_begin("db_name", ttype::STRING, 1)
            .await?;
        self.writer.write_string(db_name).await?;
        self.writer.write_field_end().await?;

        // 2: table_name
        self.writer
            .write_field_begin("table_name", ttype::STRING, 2)
            .await?;
        self.writer.write_string(table_name).await?;
        self.writer.write_field_end().await?;

        // 3: where_expr
        if let Some(w) = where_expr {
            self.writer
                .write_field_begin("where_expr", ttype::STRUCT, 3)
                .await?;
            w.write(&mut self.writer).await?;
            self.writer.write_field_end().await?;
        }

        // 4: update_expr_array
        self.writer
            .write_field_begin("update_expr_array", ttype::LIST, 4)
            .await?;
        self.writer
            .write_list_begin(ttype::STRUCT, updates.len())
            .await?;
        for upd in updates {
            upd.write(&mut self.writer).await?;
        }
        self.writer.write_list_end().await?;
        self.writer.write_field_end().await?;

        // 5: session_id
        self.writer
            .write_field_begin("session_id", ttype::I64, 5)
            .await?;
        self.writer.write_i64(self.session_id).await?;
        self.writer.write_field_end().await?;

        self.writer.write_field_stop().await?;
        self.writer.write_struct_end().await?;
        self.writer.write_field_end().await?;

        self.writer.write_field_stop().await?;
        self.writer.write_struct_end().await?;
        self.writer.write_message_end().await?;
        self.writer.flush().await?;

        let resp = self.read_common_response().await?;
        if !resp.is_ok() {
            return Err(io::Error::other(resp.error_message()));
        }
        Ok(())
    }

    pub async fn delete(
        &mut self,
        db_name: &str,
        table_name: &str,
        where_expr: Option<&ParsedExpr>,
    ) -> io::Result<()> {
        let seq_id = self.next_seq_id();
        self.writer
            .write_message_begin("Delete", MSG_CALL, seq_id)
            .await?;
        self.writer.write_struct_begin("Delete_args").await?;

        self.writer
            .write_field_begin("request", ttype::STRUCT, 1)
            .await?;
        self.writer.write_struct_begin("DeleteRequest").await?;

        // 1: db_name
        self.writer
            .write_field_begin("db_name", ttype::STRING, 1)
            .await?;
        self.writer.write_string(db_name).await?;
        self.writer.write_field_end().await?;

        // 2: table_name
        self.writer
            .write_field_begin("table_name", ttype::STRING, 2)
            .await?;
        self.writer.write_string(table_name).await?;
        self.writer.write_field_end().await?;

        // 3: where_expr
        if let Some(w) = where_expr {
            self.writer
                .write_field_begin("where_expr", ttype::STRUCT, 3)
                .await?;
            w.write(&mut self.writer).await?;
            self.writer.write_field_end().await?;
        }

        // 4: session_id
        self.writer
            .write_field_begin("session_id", ttype::I64, 4)
            .await?;
        self.writer.write_i64(self.session_id).await?;
        self.writer.write_field_end().await?;

        self.writer.write_field_stop().await?;
        self.writer.write_struct_end().await?;
        self.writer.write_field_end().await?;

        self.writer.write_field_stop().await?;
        self.writer.write_struct_end().await?;
        self.writer.write_message_end().await?;
        self.writer.flush().await?;

        let resp = self.read_common_response().await?;
        if !resp.is_ok() {
            return Err(io::Error::other(resp.error_message()));
        }
        Ok(())
    }

    pub async fn explain(
        &mut self,
        db_name: &str,
        table_name: &str,
        select_list: &[ParsedExpr],
        where_expr: Option<&ParsedExpr>,
    ) -> io::Result<SelectResponse> {
        let seq_id = self.next_seq_id();
        self.writer
            .write_message_begin("Explain", MSG_CALL, seq_id)
            .await?;
        self.writer.write_struct_begin("Explain_args").await?;

        self.writer
            .write_field_begin("request", ttype::STRUCT, 1)
            .await?;
        self.writer.write_struct_begin("ExplainRequest").await?;

        // 1: session_id
        self.writer
            .write_field_begin("session_id", ttype::I64, 1)
            .await?;
        self.writer.write_i64(self.session_id).await?;
        self.writer.write_field_end().await?;

        // 2: db_name
        self.writer
            .write_field_begin("db_name", ttype::STRING, 2)
            .await?;
        self.writer.write_string(db_name).await?;
        self.writer.write_field_end().await?;

        // 3: table_name
        self.writer
            .write_field_begin("table_name", ttype::STRING, 3)
            .await?;
        self.writer.write_string(table_name).await?;
        self.writer.write_field_end().await?;

        // 4: select_list
        self.writer
            .write_field_begin("select_list", ttype::LIST, 4)
            .await?;
        self.writer
            .write_list_begin(ttype::STRUCT, select_list.len())
            .await?;
        for expr in select_list {
            expr.write(&mut self.writer).await?;
        }
        self.writer.write_list_end().await?;
        self.writer.write_field_end().await?;

        // 7: where_expr
        if let Some(w) = where_expr {
            self.writer
                .write_field_begin("where_expr", ttype::STRUCT, 7)
                .await?;
            w.write(&mut self.writer).await?;
            self.writer.write_field_end().await?;
        }

        // 13: explain_type (Physical = 4)
        self.writer
            .write_field_begin("explain_type", ttype::I32, 13)
            .await?;
        self.writer.write_i32(ExplainType::Physical as i32).await?;
        self.writer.write_field_end().await?;

        self.writer.write_field_stop().await?;
        self.writer.write_struct_end().await?;
        self.writer.write_field_end().await?;

        self.writer.write_field_stop().await?;
        self.writer.write_struct_end().await?;
        self.writer.write_message_end().await?;
        self.writer.flush().await?;

        self.read_select_response().await
    }

    async fn read_common_response(&mut self) -> io::Result<CommonResponse> {
        let (_, msg_type, _) = self.reader.read_message_begin().await?;
        if msg_type == MSG_EXCEPTION {
            return Err(io::Error::other("Thrift application exception"));
        }

        let mut resp = CommonResponse::default();
        self.reader.read_struct_begin().await?;
        loop {
            let (f_type, f_id) = self.reader.read_field_begin().await?;
            if f_type == ttype::STOP {
                break;
            }
            if f_id == 0 {
                resp = CommonResponse::read(&mut self.reader).await?;
            } else {
                self.reader.skip(f_type, 0).await?;
            }
            self.reader.read_field_end().await?;
        }
        self.reader.read_struct_end().await?;
        self.reader.read_message_end().await?;
        Ok(resp)
    }

    async fn read_select_response(&mut self) -> io::Result<SelectResponse> {
        let (_, msg_type, _) = self.reader.read_message_begin().await?;
        if msg_type == MSG_EXCEPTION {
            return Err(io::Error::other("Thrift application exception"));
        }

        let mut resp = SelectResponse::default();
        self.reader.read_struct_begin().await?;
        loop {
            let (f_type, f_id) = self.reader.read_field_begin().await?;
            if f_type == ttype::STOP {
                break;
            }
            if f_id == 0 {
                resp = SelectResponse::read(&mut self.reader).await?;
            } else {
                self.reader.skip(f_type, 0).await?;
            }
            self.reader.read_field_end().await?;
        }
        self.reader.read_struct_end().await?;
        self.reader.read_message_end().await?;

        if !resp.is_ok() {
            return Err(io::Error::other(resp.error_message()));
        }
        Ok(resp)
    }
}

fn extract_version_index(msg: &str) -> Option<i64> {
    // E.g. "Expected version index: 37, connecting version: 1"
    // or "current max SDK version index is 37"
    if let Some(pos) = msg.find("index is ") {
        let rem = &msg[pos + 9..];
        let digits: String = rem.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(v) = digits.parse::<i64>() {
            return Some(v);
        }
    }
    if let Some(pos) = msg.find("version index: ") {
        let rem = &msg[pos + 15..];
        let digits: String = rem.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(v) = digits.parse::<i64>() {
            return Some(v);
        }
    }
    None
}
