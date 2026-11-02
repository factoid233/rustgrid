use rustgrid_core::{ConnectionConfig, Driver, DriverId, PageRequest};
use rustgrid_infinity::InfinityDriver;
use std::collections::BTreeMap;

fn config() -> ConnectionConfig {
    let host = std::env::var("RUSTGRID_INFINITY_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("RUSTGRID_INFINITY_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(23817);

    ConnectionConfig {
        driver: DriverId::new("infinity"),
        host,
        port,
        username: String::new(),
        password: None,
        database: Some("default_db".to_string()),
        options: BTreeMap::new(),
        settings: Default::default(),
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires a live Infinity server on port 23817"]
async fn live_infinity_smoke_test() {
    let driver = InfinityDriver::new();
    let conn = driver
        .connect(&config())
        .await
        .expect("connect to Infinity");

    let version = conn.server_version().await.expect("server version");
    println!("Connected to Infinity server version: {version}");

    let dbs = conn.list_databases().await.expect("list databases");
    println!("Databases: {dbs:?}");
    assert!(dbs.iter().any(|d| d.name == "default_db"));

    let tables = conn
        .list_tables("default_db")
        .await
        .expect("list tables in default_db");
    println!("Tables in default_db: {tables:?}");

    let res = conn
        .execute_query(Some("default_db"), "SHOW TABLES")
        .await
        .expect("execute SHOW TABLES");
    println!("SHOW TABLES result columns: {:?}", res.columns);

    if dbs.iter().any(|d| d.name == "flash_rag") {
        let fr_tables = conn
            .list_tables("flash_rag")
            .await
            .expect("list tables in flash_rag");
        println!("Tables in flash_rag: {fr_tables:?}");
        assert!(!fr_tables.is_empty());

        let first_tbl = &fr_tables[0].name;
        let cols = conn
            .columns("flash_rag", first_tbl)
            .await
            .expect("get columns");
        println!("Columns of flash_rag.{first_tbl}: {cols:?}");
        assert!(!cols.is_empty());

        let page = conn
            .fetch_page(
                "flash_rag",
                first_tbl,
                PageRequest {
                    page: 1,
                    page_size: 5,
                    order_by: Vec::new(),
                    filter: Vec::new(),
                    skip_total: true,
                },
            )
            .await
            .expect("fetch page");
        println!(
            "Fetched page of flash_rag.{first_tbl}: {} rows",
            page.rows.len()
        );

        // Test SELECT without LIMIT (must not fail with code 3091 total_hits_count)
        let q_no_limit = conn
            .execute_query(Some("flash_rag"), &format!("SELECT * FROM {first_tbl}"))
            .await
            .expect("execute SELECT without LIMIT");
        println!(
            "SELECT without LIMIT returned {} rows, columns: {}",
            q_no_limit.rows.len(),
            q_no_limit.columns.len()
        );

        // Test SELECT with LIMIT
        let q_with_limit = conn
            .execute_query(
                Some("flash_rag"),
                &format!("SELECT * FROM {first_tbl} LIMIT 5"),
            )
            .await
            .expect("execute SELECT with LIMIT");
        println!(
            "SELECT with LIMIT returned {} rows",
            q_with_limit.rows.len()
        );
    }
}
