use async_trait::async_trait;
use rustgrid_core::{
    BackupObjectKind, CellValue, ColumnDef as CoreColumnDef, ColumnInfo, Connection, DatabaseInfo,
    DatabaseOptions, DriverId, Error, ObjectDump, ObjectKind, PageRequest, QueryResult, Result,
    RowInsert, RowUpdate, TableInfo, TableOptions, TablePage, TableSchema, TableStatus,
};
use tokio::sync::{Mutex, RwLock};

use crate::client::InfinityClient;
use crate::decode::decode_column_field;
use crate::expr::{ParsedSqlCommand, parse_filter, parse_scalar_value, parse_sql};
use crate::types::{ColumnExpr, ConstantExpr, OrderByExpr, ParsedExpr, SelectResponse, UpdateExpr};

pub struct InfinityConnection {
    client: Mutex<InfinityClient>,
    current_db: RwLock<String>,
    _tunnel: Option<rustgrid_tunnel::Tunnel>,
}

impl InfinityConnection {
    pub fn new(
        default_db: String,
        client: InfinityClient,
        tunnel: Option<rustgrid_tunnel::Tunnel>,
    ) -> Self {
        Self {
            client: Mutex::new(client),
            current_db: RwLock::new(default_db),
            _tunnel: tunnel,
        }
    }

    fn select_response_to_query_result(resp: SelectResponse, statement: &str) -> QueryResult {
        let mut columns = Vec::new();
        let mut column_data: Vec<Vec<CellValue>> = Vec::new();
        let num_rows = resp
            .column_fields
            .as_ref()
            .and_then(|f| f.first())
            .and_then(|first| (!first.bitmasks.is_empty()).then_some(first.bitmasks.len()));

        if let Some(fields) = resp.column_fields {
            for (idx, field) in fields.into_iter().enumerate() {
                let (mut col_info, rows) = decode_column_field(&field, num_rows);
                if col_info.name.is_empty()
                    && let Some(def) = resp.column_defs.as_ref().and_then(|defs| defs.get(idx))
                {
                    col_info.name = def.name.clone();
                }
                columns.push(col_info);
                column_data.push(rows);
            }
        }

        let row_count = column_data.first().map(|c| c.len()).unwrap_or(0);
        let mut rows = Vec::with_capacity(row_count);
        for row_idx in 0..row_count {
            let mut row = Vec::with_capacity(columns.len());
            for col in &column_data {
                let cell = col.get(row_idx).cloned().unwrap_or(CellValue::Null);
                row.push(cell);
            }
            rows.push(row);
        }

        QueryResult {
            statement: statement.to_string(),
            columns,
            rows,
            rows_affected: 0,
            has_result_set: true,
            last_insert_id: None,
        }
    }
}

#[async_trait]
impl Connection for InfinityConnection {
    fn driver_id(&self) -> DriverId {
        DriverId::new("infinity")
    }

    async fn list_databases(&self) -> Result<Vec<DatabaseInfo>> {
        let mut client = self.client.lock().await;
        let names = client
            .list_databases()
            .await
            .map_err(|e| Error::Query(e.to_string()))?;

        Ok(names
            .into_iter()
            .map(|name| DatabaseInfo { name })
            .collect())
    }

    async fn list_tables(&self, database: &str) -> Result<Vec<TableInfo>> {
        let mut client = self.client.lock().await;
        let names = client
            .list_tables(database)
            .await
            .map_err(|e| Error::Query(e.to_string()))?;

        Ok(names
            .into_iter()
            .map(|name| TableInfo {
                name,
                kind: ObjectKind::Table,
                updatable: false,
            })
            .collect())
    }

    async fn columns(&self, database: &str, table: &str) -> Result<Vec<ColumnInfo>> {
        let mut client = self.client.lock().await;
        let resp = client
            .show_columns(database, table)
            .await
            .map_err(|e| Error::Query(e.to_string()))?;

        // ShowColumns returns ["name", "type", "default", "comment"] columns
        let qres = Self::select_response_to_query_result(resp, "SHOW COLUMNS");
        let mut result_columns = Vec::new();

        let name_idx = qres
            .columns
            .iter()
            .position(|c| {
                c.name.eq_ignore_ascii_case("name") || c.name.eq_ignore_ascii_case("column_name")
            })
            .unwrap_or(0);
        let type_idx = qres
            .columns
            .iter()
            .position(|c| {
                c.name.eq_ignore_ascii_case("type") || c.name.eq_ignore_ascii_case("column_type")
            })
            .unwrap_or(1);
        let comment_idx = qres
            .columns
            .iter()
            .position(|c| c.name.eq_ignore_ascii_case("comment"))
            .unwrap_or(3);

        for row in qres.rows {
            let col_name = row
                .get(name_idx)
                .map(|v| v.as_display())
                .unwrap_or_default();
            if col_name.is_empty() {
                continue;
            }
            let col_type = row
                .get(type_idx)
                .map(|v| v.as_display())
                .unwrap_or_else(|| "Varchar".to_string());
            let comment = row
                .get(comment_idx)
                .map(|v| v.as_display())
                .unwrap_or_default();

            result_columns.push(ColumnInfo {
                name: col_name.clone(),
                data_type: col_type,
                nullable: true,
                primary_key: col_name == "_row_id",
                comment,
            });
        }

        Ok(result_columns)
    }

    async fn fetch_page(
        &self,
        database: &str,
        table: &str,
        page: PageRequest,
    ) -> Result<TablePage> {
        let mut client = self.client.lock().await;

        let select_list = vec![ParsedExpr {
            column_expr: Some(ColumnExpr::star()),
            ..Default::default()
        }];

        let where_expr = parse_filter(&page.filter);

        let order_by_list = if !page.order_by.is_empty() {
            let mut list = Vec::new();
            for s in &page.order_by {
                list.push(OrderByExpr {
                    expr: ParsedExpr::column(ColumnExpr::single(&s.column)),
                    asc: !s.descending,
                });
            }
            Some(list)
        } else {
            None
        };

        let limit_expr = Some(ParsedExpr::constant(ConstantExpr::int(
            page.page_size as i64,
        )));
        let offset = ((page.page.saturating_sub(1)) * page.page_size) as i64;
        let offset_expr = Some(ParsedExpr::constant(ConstantExpr::int(offset)));

        let resp = client
            .select(
                database,
                table,
                &select_list,
                None,
                where_expr.as_ref(),
                None,
                order_by_list.as_deref(),
                limit_expr.as_ref(),
                offset_expr.as_ref(),
                !page.skip_total,
            )
            .await
            .map_err(|e| Error::Query(e.to_string()))?;

        let qres = Self::select_response_to_query_result(resp, "SELECT");
        Ok(TablePage {
            columns: qres.columns,
            rows: qres.rows,
            page: page.page,
            page_size: page.page_size,
            total_rows: None,
        })
    }

    async fn update_rows(&self, database: &str, table: &str, updates: &[RowUpdate]) -> Result<()> {
        let mut client = self.client.lock().await;
        for upd in updates {
            let mut assignments = Vec::new();
            for (col, val) in &upd.set {
                let expr = match val {
                    Some(s) => parse_scalar_value(s),
                    None => ParsedExpr::constant(ConstantExpr::null()),
                };
                assignments.push(UpdateExpr {
                    column_name: col.clone(),
                    value: expr,
                });
            }

            let mut key_exprs = Vec::new();
            for (k, val) in &upd.keys {
                let col_expr = ParsedExpr::column(ColumnExpr::single(k));
                let cond = match val {
                    Some(s) => {
                        let v = parse_scalar_value(s);
                        ParsedExpr::function(crate::types::FunctionExpr {
                            function_name: "=".to_string(),
                            arguments: Some(vec![col_expr, v]),
                        })
                    }
                    None => ParsedExpr::function(crate::types::FunctionExpr {
                        function_name: "is_null".to_string(),
                        arguments: Some(vec![col_expr]),
                    }),
                };
                key_exprs.push(cond);
            }

            let where_expr = if key_exprs.is_empty() {
                None
            } else {
                let mut iter = key_exprs.into_iter();
                let mut combined = iter.next().unwrap();
                for next_expr in iter {
                    combined = ParsedExpr::function(crate::types::FunctionExpr {
                        function_name: "and".to_string(),
                        arguments: Some(vec![combined, next_expr]),
                    });
                }
                Some(combined)
            };

            client
                .update(database, table, where_expr.as_ref(), &assignments)
                .await
                .map_err(|e| Error::Query(e.to_string()))?;
        }
        Ok(())
    }

    async fn insert_rows(&self, database: &str, table: &str, rows: &[RowInsert]) -> Result<()> {
        if rows.is_empty() {
            return Ok(());
        }

        let mut client = self.client.lock().await;
        let columns: Vec<String> = rows[0].values.iter().map(|(k, _)| k.clone()).collect();

        let mut all_rows = Vec::new();
        for r in rows {
            let mut row_exprs = Vec::new();
            for (_, val) in &r.values {
                let expr = match val {
                    Some(s) => parse_scalar_value(s),
                    None => ParsedExpr::constant(ConstantExpr::null()),
                };
                row_exprs.push(expr);
            }
            all_rows.push(row_exprs);
        }

        client
            .insert(database, table, &columns, &all_rows)
            .await
            .map_err(|e| Error::Query(e.to_string()))
    }

    async fn delete_rows(
        &self,
        database: &str,
        table: &str,
        keys: &[Vec<(String, Option<String>)>],
    ) -> Result<()> {
        let mut client = self.client.lock().await;

        for key_group in keys {
            let mut conds = Vec::new();
            for (k, val) in key_group {
                let col_expr = ParsedExpr::column(ColumnExpr::single(k));
                let cond = match val {
                    Some(s) => {
                        let v = parse_scalar_value(s);
                        ParsedExpr::function(crate::types::FunctionExpr {
                            function_name: "=".to_string(),
                            arguments: Some(vec![col_expr, v]),
                        })
                    }
                    None => ParsedExpr::function(crate::types::FunctionExpr {
                        function_name: "is_null".to_string(),
                        arguments: Some(vec![col_expr]),
                    }),
                };
                conds.push(cond);
            }

            let where_expr = if conds.is_empty() {
                None
            } else {
                let mut iter = conds.into_iter();
                let mut combined = iter.next().unwrap();
                for next_expr in iter {
                    combined = ParsedExpr::function(crate::types::FunctionExpr {
                        function_name: "and".to_string(),
                        arguments: Some(vec![combined, next_expr]),
                    });
                }
                Some(combined)
            };

            client
                .delete(database, table, where_expr.as_ref())
                .await
                .map_err(|e| Error::Query(e.to_string()))?;
        }

        Ok(())
    }

    async fn execute_query(&self, database: Option<&str>, sql: &str) -> Result<QueryResult> {
        let default_db = self.current_db.read().await.clone();
        let target_db = database.unwrap_or(&default_db);

        let parsed_cmd = parse_sql(sql).map_err(Error::Query)?;

        match parsed_cmd {
            ParsedSqlCommand::ShowDatabases => {
                let dbs = self.list_databases().await?;
                let columns = vec![ColumnInfo {
                    name: "Database".to_string(),
                    data_type: "Varchar".to_string(),
                    nullable: false,
                    primary_key: false,
                    comment: String::new(),
                }];
                let rows = dbs
                    .into_iter()
                    .map(|db| vec![CellValue::Text(db.name)])
                    .collect();
                Ok(QueryResult {
                    statement: sql.to_string(),
                    columns,
                    rows,
                    rows_affected: 0,
                    has_result_set: true,
                    last_insert_id: None,
                })
            }
            ParsedSqlCommand::ShowTables { db_name } => {
                let db = db_name.as_deref().unwrap_or(target_db);
                let mut client = self.client.lock().await;
                let resp = client
                    .show_tables(db)
                    .await
                    .map_err(|e| Error::Query(e.to_string()))?;
                Ok(Self::select_response_to_query_result(resp, sql))
            }
            ParsedSqlCommand::ShowColumns {
                db_name,
                table_name,
            } => {
                let db = db_name.as_deref().unwrap_or(target_db);
                let mut client = self.client.lock().await;
                let resp = client
                    .show_columns(db, &table_name)
                    .await
                    .map_err(|e| Error::Query(e.to_string()))?;
                Ok(Self::select_response_to_query_result(resp, sql))
            }
            ParsedSqlCommand::Use { db_name } => {
                let mut client = self.client.lock().await;
                client
                    .use_database(&db_name)
                    .await
                    .map_err(|e| Error::Query(e.to_string()))?;
                *self.current_db.write().await = db_name;
                Ok(QueryResult {
                    statement: sql.to_string(),
                    columns: Vec::new(),
                    rows: Vec::new(),
                    rows_affected: 0,
                    has_result_set: false,
                    last_insert_id: None,
                })
            }
            ParsedSqlCommand::Select {
                db_name,
                table_name,
                select_list,
                search_expr,
                where_expr,
                group_by_list,
                order_by_list,
                limit_expr,
                offset_expr,
            } => {
                let db = db_name.as_deref().unwrap_or(target_db);
                let has_limit = limit_expr.is_some();
                let mut client = self.client.lock().await;
                let resp = client
                    .select(
                        db,
                        &table_name,
                        &select_list,
                        search_expr.as_ref(),
                        where_expr.as_ref(),
                        group_by_list.as_deref(),
                        order_by_list.as_deref(),
                        limit_expr.as_ref(),
                        offset_expr.as_ref(),
                        has_limit,
                    )
                    .await
                    .map_err(|e| Error::Query(e.to_string()))?;
                Ok(Self::select_response_to_query_result(resp, sql))
            }
            ParsedSqlCommand::Insert {
                db_name,
                table_name,
                columns,
                values,
            } => {
                let db = db_name.as_deref().unwrap_or(target_db);
                let row_count = values.len() as u64;
                let mut client = self.client.lock().await;
                client
                    .insert(db, &table_name, &columns, &values)
                    .await
                    .map_err(|e| Error::Query(e.to_string()))?;
                Ok(QueryResult {
                    statement: sql.to_string(),
                    columns: Vec::new(),
                    rows: Vec::new(),
                    rows_affected: row_count,
                    has_result_set: false,
                    last_insert_id: None,
                })
            }
            ParsedSqlCommand::Update {
                db_name,
                table_name,
                assignments,
                where_expr,
            } => {
                let db = db_name.as_deref().unwrap_or(target_db);
                let mut client = self.client.lock().await;
                client
                    .update(db, &table_name, where_expr.as_ref(), &assignments)
                    .await
                    .map_err(|e| Error::Query(e.to_string()))?;
                Ok(QueryResult {
                    statement: sql.to_string(),
                    columns: Vec::new(),
                    rows: Vec::new(),
                    rows_affected: 1,
                    has_result_set: false,
                    last_insert_id: None,
                })
            }
            ParsedSqlCommand::Delete {
                db_name,
                table_name,
                where_expr,
            } => {
                let db = db_name.as_deref().unwrap_or(target_db);
                let mut client = self.client.lock().await;
                client
                    .delete(db, &table_name, where_expr.as_ref())
                    .await
                    .map_err(|e| Error::Query(e.to_string()))?;
                Ok(QueryResult {
                    statement: sql.to_string(),
                    columns: Vec::new(),
                    rows: Vec::new(),
                    rows_affected: 1,
                    has_result_set: false,
                    last_insert_id: None,
                })
            }
            ParsedSqlCommand::CreateTable {
                db_name,
                table_name,
                columns,
            } => {
                let db = db_name.as_deref().unwrap_or(target_db);
                let mut client = self.client.lock().await;
                client
                    .create_table(db, &table_name, &columns, None)
                    .await
                    .map_err(|e| Error::Query(e.to_string()))?;
                Ok(QueryResult {
                    statement: sql.to_string(),
                    columns: Vec::new(),
                    rows: Vec::new(),
                    rows_affected: 0,
                    has_result_set: false,
                    last_insert_id: None,
                })
            }
            ParsedSqlCommand::DropTable {
                db_name,
                table_name,
            } => {
                let db = db_name.as_deref().unwrap_or(target_db);
                let mut client = self.client.lock().await;
                client
                    .drop_table(db, &table_name)
                    .await
                    .map_err(|e| Error::Query(e.to_string()))?;
                Ok(QueryResult {
                    statement: sql.to_string(),
                    columns: Vec::new(),
                    rows: Vec::new(),
                    rows_affected: 0,
                    has_result_set: false,
                    last_insert_id: None,
                })
            }
            ParsedSqlCommand::CreateDatabase { db_name } => {
                let mut client = self.client.lock().await;
                client
                    .create_database(&db_name, None)
                    .await
                    .map_err(|e| Error::Query(e.to_string()))?;
                Ok(QueryResult {
                    statement: sql.to_string(),
                    columns: Vec::new(),
                    rows: Vec::new(),
                    rows_affected: 0,
                    has_result_set: false,
                    last_insert_id: None,
                })
            }
            ParsedSqlCommand::DropDatabase { db_name } => {
                let mut client = self.client.lock().await;
                client
                    .drop_database(&db_name)
                    .await
                    .map_err(|e| Error::Query(e.to_string()))?;
                Ok(QueryResult {
                    statement: sql.to_string(),
                    columns: Vec::new(),
                    rows: Vec::new(),
                    rows_affected: 0,
                    has_result_set: false,
                    last_insert_id: None,
                })
            }
            ParsedSqlCommand::Explain {
                db_name,
                table_name,
                select_list,
                where_expr,
            } => {
                let db = db_name.as_deref().unwrap_or(target_db);
                let mut client = self.client.lock().await;
                let resp = client
                    .explain(db, &table_name, &select_list, where_expr.as_ref())
                    .await
                    .map_err(|e| Error::Query(e.to_string()))?;
                Ok(Self::select_response_to_query_result(resp, sql))
            }
        }
    }

    async fn create_database(&self, name: &str, _options: &DatabaseOptions) -> Result<()> {
        let mut client = self.client.lock().await;
        client
            .create_database(name, None)
            .await
            .map_err(|e| Error::Query(e.to_string()))
    }

    fn create_database_sql(&self, name: &str, _options: &DatabaseOptions) -> String {
        format!("CREATE DATABASE `{name}`;")
    }

    async fn drop_database(&self, name: &str) -> Result<()> {
        let mut client = self.client.lock().await;
        client
            .drop_database(name)
            .await
            .map_err(|e| Error::Query(e.to_string()))
    }

    async fn drop_table(&self, database: &str, table: &str) -> Result<()> {
        let mut client = self.client.lock().await;
        client
            .drop_table(database, table)
            .await
            .map_err(|e| Error::Query(e.to_string()))
    }

    async fn empty_table(&self, database: &str, table: &str) -> Result<()> {
        let mut client = self.client.lock().await;
        client
            .delete(database, table, None)
            .await
            .map_err(|e| Error::Query(e.to_string()))
    }

    async fn truncate_table(&self, database: &str, table: &str) -> Result<()> {
        self.empty_table(database, table).await
    }

    async fn rename_table(&self, _database: &str, _table: &str, _new_name: &str) -> Result<()> {
        Err(Error::Query(
            "table rename is not supported by Infinity".to_string(),
        ))
    }

    async fn database_options(&self, _name: &str) -> Result<DatabaseOptions> {
        Ok(DatabaseOptions::default())
    }

    async fn server_version(&self) -> Result<String> {
        let client = self.client.lock().await;
        Ok(client.server_version().to_string())
    }

    async fn session_count(&self) -> Result<u64> {
        Ok(1)
    }

    async fn table_status(&self, _database: &str, _table: &str) -> Result<TableStatus> {
        Ok(TableStatus::default())
    }

    async fn character_sets(&self) -> Result<Vec<String>> {
        Ok(vec!["utf8mb4".to_string()])
    }

    async fn collations(&self) -> Result<Vec<String>> {
        Ok(vec!["utf8mb4_bin".to_string()])
    }

    async fn alter_database_options(
        &self,
        _name: &str,
        _original: &DatabaseOptions,
        _modified: &DatabaseOptions,
    ) -> Result<()> {
        Ok(())
    }

    fn alter_database_sql(
        &self,
        _name: &str,
        _original: &DatabaseOptions,
        _modified: &DatabaseOptions,
    ) -> String {
        String::new()
    }

    fn column_types(&self) -> Vec<&'static str> {
        vec![
            "Boolean",
            "TinyInt",
            "SmallInt",
            "Integer",
            "BigInt",
            "Float",
            "Double",
            "Varchar",
            "Embedding(float32, 128)",
            "Date",
            "Time",
            "DateTime",
            "Timestamp",
            "Json",
        ]
    }

    async fn list_routines(&self, _database: &str) -> Result<Vec<String>> {
        Ok(Vec::new())
    }

    async fn list_events(&self, _database: &str) -> Result<Vec<String>> {
        Ok(Vec::new())
    }

    async fn backup_object_metadata(
        &self,
        _database: &str,
        _kind: BackupObjectKind,
        _name: &str,
    ) -> Result<ObjectDump> {
        Ok(ObjectDump {
            name: String::new(),
            kind: BackupObjectKind::Table,
            ddl: String::new(),
            fields: Vec::new(),
            trigger_ddl: Vec::new(),
            rows: Vec::new(),
        })
    }

    async fn stream_table_rows(
        &self,
        _database: &str,
        _table: &str,
        _on_row: &mut (dyn for<'a> FnMut(&'a str) -> Result<()> + Send),
    ) -> Result<u64> {
        Ok(0)
    }

    async fn restore_object(&self, _database: &str, _object: &ObjectDump) -> Result<()> {
        Ok(())
    }

    fn storage_engines(&self) -> Vec<&'static str> {
        Vec::new()
    }

    async fn table_schema(&self, database: &str, table: &str) -> Result<TableSchema> {
        let cols = self.columns(database, table).await?;
        let mut defs = Vec::new();
        for c in cols {
            defs.push(CoreColumnDef {
                name: c.name,
                data_type: c.data_type,
                nullable: c.nullable,
                primary_key: c.primary_key,
                comment: c.comment,
                ..Default::default()
            });
        }
        Ok(TableSchema {
            columns: defs,
            indexes: Vec::new(),
            unique_keys: Vec::new(),
            checks: Vec::new(),
            excludes: Vec::new(),
            rules: Vec::new(),
            foreign_keys: Vec::new(),
            triggers: Vec::new(),
            partitioning: String::new(),
            options: TableOptions::default(),
        })
    }

    fn table_schema_sql(
        &self,
        _database: &str,
        _table: &str,
        _original: Option<&TableSchema>,
        _modified: &TableSchema,
    ) -> String {
        String::new()
    }

    async fn close(&self) -> Result<()> {
        let mut client = self.client.lock().await;
        let _ = client.disconnect().await;
        Ok(())
    }
}
