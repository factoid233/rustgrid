use std::time::Duration;

use async_trait::async_trait;
use rustgrid_core::{
    AdvancedFields, Connection, ConnectionConfig, ConnectionFieldLabel, ConnectionFormSpec,
    ConnectionHomePage, ConnectionPage, ConnectionStandardField, Driver, DriverCapabilities,
    DriverCapability, DriverDescriptor, DriverIconStyle, DriverId, Error, Result,
    TableDesignerSpec,
};
use tokio::net::TcpStream;
use tokio::time::timeout;

use crate::client::{BoxedRead, BoxedWrite, InfinityClient};
use crate::connection::InfinityConnection;

#[derive(Debug, Default)]
pub struct InfinityDriver;

impl InfinityDriver {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Driver for InfinityDriver {
    fn id(&self) -> DriverId {
        DriverId::new("infinity")
    }

    fn display_name(&self) -> String {
        "Infinity".to_string()
    }

    fn default_port(&self) -> u16 {
        23817
    }

    fn supports_database_management(&self) -> bool {
        true
    }

    fn supports_users(&self) -> bool {
        false
    }

    fn supports_routines(&self) -> bool {
        false
    }

    fn supports_schemas(&self) -> bool {
        false
    }

    fn supports_table_designer(&self, _profile: &rustgrid_core::ConnectionProfile) -> bool {
        false
    }

    fn supports_backup(&self, _profile: &rustgrid_core::ConnectionProfile) -> bool {
        false
    }

    fn descriptor(&self) -> DriverDescriptor {
        DriverDescriptor {
            id: self.id(),
            display_name: self.display_name(),
            default_port: self.default_port(),
            is_file_based: false,
            icon: "icons/infinity.svg",
            icon_style: DriverIconStyle::Brand(0x2862FF),
            capabilities: DriverCapabilities::none().with(DriverCapability::DatabaseManagement),
            database_editor: Default::default(),
            connection_form: ConnectionFormSpec {
                home: ConnectionHomePage::Network,
                tabs: vec![ConnectionPage::Tunnel, ConnectionPage::Advanced],
                optional_fields: vec![
                    ConnectionStandardField::Database,
                    ConnectionStandardField::Username,
                    ConnectionStandardField::Password,
                ],
                labels: vec![ConnectionFieldLabel {
                    field: ConnectionStandardField::Database,
                    label_key: "form.database",
                    hint_key: None,
                    visible_when: None,
                }],
                options: Vec::new(),
                advanced: AdvancedFields {
                    connect_timeout: true,
                    query_timeout: true,
                    keepalive: false,
                    read_only: false,
                    init_sql: false,
                },
            },
            table_designer: TableDesignerSpec::default(),
            order: 75,
        }
    }

    async fn connect(&self, config: &ConnectionConfig) -> Result<Box<dyn Connection>> {
        let host = if config.host.trim().is_empty() {
            "127.0.0.1".to_string()
        } else {
            config.host.clone()
        };
        let port = if config.port == 0 {
            self.default_port()
        } else {
            config.port
        };

        let (target_host, target_port, tunnel) = if !config.settings.tunnel.is_empty() {
            let tun = rustgrid_tunnel::Tunnel::start(config.settings.tunnel.clone(), host, port)
                .await
                .map_err(|e| Error::Connection(format!("Tunnel error: {e}")))?;
            let local_port = tun.local_port();
            ("127.0.0.1".to_string(), local_port, Some(tun))
        } else {
            (host, port, None)
        };

        let connect_timeout_duration =
            Duration::from_secs(config.settings.connect_timeout.unwrap_or(10).max(1));

        let tcp_stream = timeout(
            connect_timeout_duration,
            TcpStream::connect((target_host.as_str(), target_port)),
        )
        .await
        .map_err(|_| {
            Error::Connection(format!(
                "Connection timeout after {connect_timeout_duration:?}"
            ))
        })?
        .map_err(|e| Error::Connection(format!("Failed to connect to Infinity: {e}")))?;

        let (read_half, write_half) = tcp_stream.into_split();
        let boxed_read: BoxedRead = Box::new(read_half);
        let boxed_write: BoxedWrite = Box::new(write_half);

        let client = timeout(
            connect_timeout_duration,
            InfinityClient::connect(
                boxed_read,
                boxed_write,
                Some(config.username.clone()).filter(|u| !u.is_empty()),
                config.password.clone(),
            ),
        )
        .await
        .map_err(|_| Error::Connection("Infinity handshake timed out".to_string()))?
        .map_err(|e| Error::Connection(format!("Infinity handshake failed: {e}")))?;

        let default_db = config
            .database
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "default_db".to_string());
        let conn = InfinityConnection::new(default_db, client, tunnel);
        Ok(Box::new(conn))
    }
}
