//! Client Manager for EasyTier with MySQL storage
//!
//! This module provides client management functionality compatible with easytier-web,
//! but using MySQL instead of SQLite for data persistence.

use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
};

use anyhow::{self, Context};
use dashmap::DashMap;
use easytier::{
    common::network::{local_ipv4, local_ipv6},
    proto::web::HeartbeatRequest,
    tunnel::{
        tcp::TcpTunnelListener, udp::UdpTunnelListener, websocket::WSTunnelListener, TunnelListener,
    },
};
#[cfg(feature = "server")]
use maxminddb::geoip2;
use tokio::task::JoinSet;

use crate::db::Database;

pub mod session;
pub mod storage;

use session::{Location, Session};
use storage::{Storage, StorageToken};

pub type OrgIdInDb = i32;

#[derive(Debug)]
pub enum Error {
    InvalidUrl(String),
    ListenerError(anyhow::Error),
    DatabaseError(anyhow::Error),
    NetworkError(anyhow::Error),
}

impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Self {
        Error::ListenerError(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::NetworkError(err.into())
    }
}

/// Create a TunnelListener from URL
pub fn get_listener_by_url(l: &url::Url) -> Result<Box<dyn TunnelListener>, Error> {
    Ok(match l.scheme() {
        "tcp" => Box::new(TcpTunnelListener::new(l.clone())),
        "udp" => Box::new(UdpTunnelListener::new(l.clone())),
        "ws" => Box::new(WSTunnelListener::new(l.clone())),
        _ => {
            return Err(Error::InvalidUrl(l.to_string()));
        }
    })
}

/// Create dual-stack listeners (IPv4 and IPv6) for a given protocol and port
pub async fn get_dual_stack_listener(
    protocol: &str,
    port: u16,
) -> Result<
    (
        Option<Box<dyn TunnelListener>>,
        Option<Box<dyn TunnelListener>>,
    ),
    Error,
> {
    let is_protocol_support_dual_stack =
        protocol.trim().to_lowercase() == "tcp" || protocol.trim().to_lowercase() == "udp";
    let v6_listener = if is_protocol_support_dual_stack && local_ipv6().await.is_ok() {
        get_listener_by_url(&format!("{protocol}://[::0]:{port}").parse().unwrap()).ok()
    } else {
        None
    };
    let v4_listener = if local_ipv4().await.is_ok() {
        get_listener_by_url(&format!("{protocol}://0.0.0.0:{port}").parse().unwrap()).ok()
    } else {
        None
    };
    Ok((v6_listener, v4_listener))
}

#[cfg(feature = "server")]
fn load_geoip_db(geoip_db: Option<String>) -> Option<maxminddb::Reader<Vec<u8>>> {
    if let Some(path) = geoip_db {
        crate::info!("[GEOIP] Attempting to load GeoIP2 database from: {}", path);
        match maxminddb::Reader::open_readfile(&path) {
            Ok(reader) => {
                crate::info!("[GEOIP] Successfully loaded GeoIP2 database from: {}", path);
                Some(reader)
            }
            Err(err) => {
                crate::warn!(
                    "[GEOIP] Failed to load GeoIP2 database from {}: {}",
                    path,
                    err
                );
                None
            }
        }
    } else {
        crate::info!("[GEOIP] No GeoIP2 database path provided, GeoIP lookup will be disabled");
        None
    }
}

#[derive(Debug)]
pub struct ClientManager {
    tasks: JoinSet<()>,
    listeners_cnt: Arc<AtomicU32>,
    client_sessions: Arc<DashMap<url::Url, Arc<Session>>>,
    storage: Storage,
    #[cfg(feature = "server")]
    geoip_db: Arc<Option<maxminddb::Reader<Vec<u8>>>>,
}

/// Run database migrations to create required tables
pub async fn run_migrations(conn: &sea_orm::DatabaseConnection) -> Result<(), String> {
    use crate::db::migrations::Migrator;
    use sea_orm_migration::MigratorTrait;

    crate::debug!("Running database migrations");
    match Migrator::up(conn, None).await {
        Ok(_) => {
            crate::debug!("Database migrations completed successfully");
            Ok(())
        }
        Err(e) => {
            crate::error!("Database migrations failed: {}", e);
            Err(format!("Migration failed: {}", e))
        }
    }
}

/// Open a database connection and run migrations
async fn open(database_url: &str) -> Result<Database, Error> {
    crate::debug!("Connecting to database: {}", database_url);
    let database = match Database::new(database_url).await {
        Ok(db) => db,
        Err(e) => {
            crate::error!("Database connection failed: {}", e);
            return Err(Error::DatabaseError(anyhow::anyhow!(
                "Database connection failed: {}",
                e
            )));
        }
    };

    // Check if required tables exist and run migrations if needed
    let conn = database.orm();
    // Try to run migrations
    if let Err(e) = run_migrations(conn).await {
        crate::error!("Failed to run migrations: {}", e);
        crate::error!("Required database tables do not exist and migrations failed. ClientManager initialization aborted.");
        return Err(Error::DatabaseError(anyhow::anyhow!(
            "Failed to run migrations: {}",
            e
        )));
    }

    Ok(database)
}

impl ClientManager {
    /// Create a new ClientManager with MySQL database
    ///
    /// # Arguments
    /// * `db_url` - Database connection URL
    /// * `geoip_db` - Optional path to GeoIP database. If None, it will try to auto-detect from project resources
    ///
    /// # Returns
    /// * `Result<Self, Error>` - New ClientManager instance or error
    pub async fn new(db_url: &str, geoip_db: Option<String>) -> Result<Self, Error> {
        crate::info!("[CLIENT_MANAGER] Initializing ClientManager with MySQL database");

        // Initialize database connection and run migrations
        let database = open(db_url).await?;

        let client_sessions = Arc::new(DashMap::new());
        let sessions: Arc<DashMap<url::Url, Arc<Session>>> = client_sessions.clone();
        let mut tasks = JoinSet::new();

        // Cleanup task for inactive sessions
        crate::debug!("[CLIENT_MANAGER] Starting cleanup task for inactive sessions");
        tasks.spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(15)).await;
                let initial_count = sessions.len();
                sessions.retain(|_, session| session.is_running());
                let final_count = sessions.len();
                if initial_count != final_count {
                    crate::debug!(
                        "[CLIENT_MANAGER] Cleaned up {} inactive sessions (from {} to {})",
                        initial_count - final_count,
                        initial_count,
                        final_count
                    );
                }
            }
        });

        // Device timeout task - mark devices as offline if no heartbeat for 60 seconds
        let storage_weak = Storage::new(database.clone()).weak_ref();
        tasks.spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;

                if let Ok(storage) = Storage::try_from(storage_weak.clone()) {
                    if let Err(e) = Self::mark_offline_devices(&storage).await {
                        crate::error!("[CLIENT_MANAGER] Failed to mark offline devices: {:?}", e);
                    }
                }
            }
        });

        // Use provided path or auto-detect from configuration
        #[cfg(feature = "server")]
        let geoip_path = geoip_db.or_else(crate::config::get_geoip_db_path);

        let manager = ClientManager {
            tasks,
            listeners_cnt: Arc::new(AtomicU32::new(0)),
            client_sessions,
            storage: Storage::new(database),
            #[cfg(feature = "server")]
            geoip_db: Arc::new(load_geoip_db(geoip_path)),
        };

        crate::info!("[CLIENT_MANAGER] ClientManager initialized successfully");
        Ok(manager)
    }

    pub async fn start(&mut self, protocol: &str, port: u16) -> Result<(), anyhow::Error> {
        // Get dual-stack listeners
        let (v6_listener, v4_listener) = get_dual_stack_listener(protocol, port)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get dual stack listener: {:?}", e))?;

        // Check if at least one listener is available
        if v4_listener.is_none() && v6_listener.is_none() {
            return Err(anyhow::anyhow!("Failed to listen on both IPv4 and IPv6"));
        }

        // Add IPv6 listener
        if let Some(listener) = v6_listener {
            self.add_listener(listener).await?;
        }

        // Add IPv4 listener
        if let Some(listener) = v4_listener {
            self.add_listener(listener).await?;
        }

        Ok(())
    }
    /// Add a tunnel listener
    pub async fn add_listener<L: TunnelListener + 'static>(
        &mut self,
        mut listener: L,
    ) -> Result<(), anyhow::Error> {
        crate::info!("[CLIENT_MANAGER] Adding new tunnel listener");

        listener.listen().await.map_err(|e| {
            crate::error!("[CLIENT_MANAGER] Failed to start listener: {:?}", e);
            e
        })?;

        let listener_id = self.listeners_cnt.fetch_add(1, Ordering::Relaxed) + 1;
        crate::info!(
            "[CLIENT_MANAGER] Tunnel listener {} started successfully",
            listener_id
        );

        let sessions = self.client_sessions.clone();
        let storage = self.storage.weak_ref();
        let listeners_cnt = self.listeners_cnt.clone();
        #[cfg(feature = "server")]
        let geoip_db = self.geoip_db.clone();

        self.tasks.spawn(async move {
            crate::debug!(
                "[CLIENT_MANAGER] Listener {} task started, waiting for connections",
                listener_id
            );

            while let Ok(tunnel) = listener.accept().await {
                let info = tunnel.info().unwrap();
                let client_url: url::Url = info.remote_addr.unwrap().into();
                #[cfg(feature = "server")]
                let location = Self::lookup_location(&client_url, geoip_db.clone());
                #[cfg(not(feature = "server"))]
                let location = None;

                crate::info!(
                    "[CLIENT_MANAGER] New client connected from {} (listener {})",
                    client_url,
                    listener_id
                );

                let mut session = Session::new(storage.clone(), client_url.clone(), location);
                session.serve(tunnel).await;
                sessions.insert(client_url.clone(), Arc::new(session));

                crate::trace!(
                    "[CLIENT_MANAGER] Session {} added to active sessions (total: {})",
                    client_url,
                    sessions.len()
                );
            }

            listeners_cnt.fetch_sub(1, Ordering::Relaxed);
            crate::info!("[CLIENT_MANAGER] Listener {} task terminated", listener_id);
        });

        Ok(())
    }

    /// Check if the client manager is running
    pub fn is_running(&self) -> bool {
        self.listeners_cnt.load(Ordering::Relaxed) > 0
    }

    /// List all active sessions
    pub async fn list_sessions(&self) -> Vec<StorageToken> {
        crate::debug!("[CLIENT_MANAGER] Listing all active sessions");

        let sessions = self
            .client_sessions
            .iter()
            .map(|item| item.value().clone())
            .collect::<Vec<_>>();

        let mut ret: Vec<StorageToken> = vec![];
        for s in sessions {
            if let Some(t) = s.get_token().await {
                ret.push(t);
            }
        }

        crate::debug!("[CLIENT_MANAGER] Found {} active sessions", ret.len());
        ret
    }

    /// Get session by device ID
    pub async fn get_session_by_device_id(
        &self,
        organization_id: &str,
        device_id: &uuid::Uuid,
    ) -> Option<Arc<Session>> {
        crate::debug!(
            "[CLIENT_MANAGER] Getting session for organization_id: {}, device_id: {}",
            organization_id,
            device_id
        );

        let c_url = self
            .storage
            .get_client_url_by_device_id(&organization_id.to_string(), device_id)?;

        let parsed_url = c_url;
        let session = self
            .client_sessions
            .get(&parsed_url)
            .map(|item| item.value().clone());

        if session.is_some() {
            crate::debug!(
                "[CLIENT_MANAGER] Found session for device_id: {}",
                device_id
            );
        } else {
            crate::debug!(
                "[CLIENT_MANAGER] No active session found for device_id: {}",
                device_id
            );
        }

        session
    }

    /// List devices by organization ID
    pub async fn list_devices_by_organization_id(&self, organization_id: &str) -> Vec<url::Url> {
        crate::debug!(
            "[CLIENT_MANAGER] Listing devices for organization_id: {}",
            organization_id
        );

        let urls = self
            .storage
            .list_organization_clients(&organization_id.to_string());
        crate::info!(
            "[CLIENT_MANAGER] Found {} devices for organization_id: {}",
            urls.len(),
            organization_id
        );
        urls
    }

    /// Get heartbeat requests for a client
    pub async fn get_heartbeat_requests(&self, client_url: &url::Url) -> Option<HeartbeatRequest> {
        crate::trace!(
            "[CLIENT_MANAGER] Getting heartbeat request for client: {}",
            client_url
        );

        let session = self.client_sessions.get(client_url)?.value().clone();
        let heartbeat = session.data().read().await.req();

        if heartbeat.is_some() {
            crate::trace!(
                "[CLIENT_MANAGER] Found heartbeat request for client: {}",
                client_url
            );
        } else {
            crate::trace!(
                "[CLIENT_MANAGER] No heartbeat request found for client: {}",
                client_url
            );
        }

        heartbeat
    }

    /// Get device location
    pub async fn get_device_location(&self, client_url: &url::Url) -> Option<Location> {
        crate::trace!(
            "[CLIENT_MANAGER] Getting location for client: {}",
            client_url
        );

        let session = self.client_sessions.get(client_url)?.value().clone();
        let location = session.data().read().await.location().cloned();

        if let Some(ref loc) = location {
            crate::trace!(
                "[CLIENT_MANAGER] Found location for client {}: {:?}",
                client_url,
                loc
            );
        } else {
            crate::trace!(
                "[CLIENT_MANAGER] No location found for client: {}",
                client_url
            );
        }

        location
    }

    /// Get database reference
    pub async fn db(&self) -> Database {
        self.storage.db().clone()
    }

    /// Get storage reference for testing
    pub fn storage(&self) -> &Storage {
        &self.storage
    }

    /// Mark devices as offline if they haven't sent heartbeat for more than 60 seconds
    async fn mark_offline_devices(storage: &Storage) -> Result<(), anyhow::Error> {
        use crate::db::entities::devices;
        use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

        let cutoff_time = chrono::Utc::now() - chrono::Duration::seconds(60);

        crate::debug!(
            "[CLIENT_MANAGER] Checking for offline devices, cutoff_time: {:?}",
            cutoff_time
        );

        // Find devices that haven't sent heartbeat recently and are approved but not already offline
        // Only mark approved devices as offline - pending/rejected devices should maintain their status
        let offline_devices = devices::Entity::find()
            .filter(devices::Column::LastHeartbeat.lt(cutoff_time))
            .filter(devices::Column::Status.ne(devices::DeviceStatus::Offline))
            .filter(devices::Column::Status.eq(devices::DeviceStatus::Approved))
            .all(storage.db().orm())
            .await
            .with_context(|| "Failed to query devices for timeout check")?;

        if offline_devices.is_empty() {
            crate::debug!("[CLIENT_MANAGER] No devices to mark as offline");
            return Ok(());
        }

        crate::info!(
            "[CLIENT_MANAGER] Marking {} devices as offline due to timeout",
            offline_devices.len()
        );

        // Log each device being marked offline
        for device in &offline_devices {
            crate::info!(
                "[CLIENT_MANAGER] Device {} ({}) last_heartbeat: {:?}, status: {:?}",
                device.id,
                device.name,
                device.last_heartbeat,
                device.status
            );
        }

        // Mark each device as offline
        for device in offline_devices {
            let mut active: devices::ActiveModel = device.clone().into();
            active.status = Set(devices::DeviceStatus::Offline);
            active.updated_at = Set(chrono::Utc::now().into());

            active
                .update(storage.db().orm())
                .await
                .with_context(|| format!("Failed to mark device {} as offline", device.id))?;

            crate::debug!(
                "[CLIENT_MANAGER] Marked device {} as offline due to timeout",
                device.id
            );
        }

        Ok(())
    }

    /// Shutdown the client manager and cleanup resources
    pub async fn shutdown(&mut self) {
        crate::info!("[CLIENT_MANAGER] Shutting down ClientManager...");

        let active_sessions = self.client_sessions.len();
        let active_listeners = self.listeners_cnt.load(Ordering::Relaxed);

        crate::info!(
            "[CLIENT_MANAGER] Shutdown initiated - {} active sessions, {} active listeners",
            active_sessions,
            active_listeners
        );

        self.tasks.shutdown().await;

        crate::info!("[CLIENT_MANAGER] ClientManager shutdown completed");
    }

    /// Lookup geographic location for client IP
    #[cfg(feature = "server")]
    fn lookup_location(
        client_url: &url::Url,
        geoip_db: Arc<Option<maxminddb::Reader<Vec<u8>>>>,
    ) -> Option<Location> {
        let host = client_url.host_str()?;
        crate::trace!("[GEOIP] Looking up location for host: {}", host);

        let ip: std::net::IpAddr = if let Ok(ip) = host.parse() {
            ip
        } else {
            crate::debug!("[GEOIP] Failed to parse host as IP address: {}", host);
            return None;
        };

        // Skip lookup for private/special IPs
        let is_private = match ip {
            std::net::IpAddr::V4(ipv4) => {
                ipv4.is_private() || ipv4.is_loopback() || ipv4.is_unspecified()
            }
            std::net::IpAddr::V6(ipv6) => ipv6.is_loopback() || ipv6.is_unspecified(),
        };

        if is_private {
            crate::debug!(
                "[GEOIP] Skipping GeoIP lookup for private/special IP: {}",
                ip
            );
            let location = Location {
                country: "本地网络".to_string(),
                city: None,
                region: None,
            };
            return Some(location);
        }

        let location = if let Some(db) = &*geoip_db {
            crate::trace!("[GEOIP] Performing GeoIP lookup for IP: {}", ip);
            match db.lookup::<geoip2::City>(ip) {
                Ok(city) => {
                    let country = city
                        .country
                        .and_then(|c| c.names)
                        .and_then(|n| {
                            n.get("zh-CN")
                                .or_else(|| n.get("en"))
                                .map(|s| s.to_string())
                        })
                        .unwrap_or_else(|| "海外".to_string());

                    let city_name = city.city.and_then(|c| c.names).and_then(|n| {
                        n.get("zh-CN")
                            .or_else(|| n.get("en"))
                            .map(|s| s.to_string())
                    });

                    let region = city
                        .subdivisions
                        .and_then(|mut subdivisions| subdivisions.pop())
                        .and_then(|subdivision| subdivision.names)
                        .and_then(|n| {
                            n.get("zh-CN")
                                .or_else(|| n.get("en"))
                                .map(|s| s.to_string())
                        });

                    let location = Location {
                        country: country.clone(),
                        city: city_name.clone(),
                        region: region.clone(),
                    };

                    crate::debug!("[GEOIP] Successfully resolved location for {}: country={}, city={:?}, region={:?}", 
                                  ip, country, city_name, region);
                    location
                }
                Err(err) => {
                    crate::debug!("[GEOIP] GeoIP lookup failed for {}: {}", ip, err);
                    Location {
                        country: "未知".to_string(),
                        city: None,
                        region: None,
                    }
                }
            }
        } else {
            crate::trace!("[GEOIP] No GeoIP database available, returning unknown location");
            Location {
                country: "未知".to_string(),
                city: None,
                region: None,
            }
        };

        Some(location)
    }
}
