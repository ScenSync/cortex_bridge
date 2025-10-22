//! Storage management for EasyTier clients with MySQL backend

use std::sync::Arc;

use dashmap::DashMap;
use uuid::Uuid;

use crate::db::{Database, OrgIdInDb};

/// Storage token for client identification
/// Updated to align with cortex_server models: machines -> devices, user_id -> organization_id
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StorageToken {
    pub token: String,
    pub client_url: url::Url,
    pub device_id: Uuid, // Changed from machine_id to device_id to align with cortex_server Device model
    pub organization_id: OrgIdInDb, // Changed from user_id to organization_id to align with cortex_server Organization model
}

#[derive(Debug, Clone)]
struct ClientInfo {
    storage_token: StorageToken,
    report_time: i64,
}

/// Weak reference to storage for avoiding circular references
pub type WeakRefStorage = std::sync::Weak<StorageInner>;

/// Internal storage data
#[derive(Debug)]
pub struct StorageInner {
    // some map for indexing
    org_clients_map: DashMap<OrgIdInDb, DashMap<uuid::Uuid, ClientInfo>>,
    pub db: Database,
}

/// Storage implementation
#[derive(Debug, Clone)]
pub struct Storage(Arc<StorageInner>);

impl TryFrom<WeakRefStorage> for Storage {
    type Error = ();

    fn try_from(weak: WeakRefStorage) -> Result<Self, Self::Error> {
        weak.upgrade().map(Storage).ok_or(())
    }
}

impl Storage {
    pub fn new(db: Database) -> Self {
        Storage(Arc::new(StorageInner {
            org_clients_map: DashMap::new(),
            db,
        }))
    }

    fn remove_device_to_client_info_map(
        map: &DashMap<uuid::Uuid, ClientInfo>,
        device_id: &uuid::Uuid,
        client_url: &url::Url,
    ) {
        map.remove_if(device_id, |_, v| v.storage_token.client_url == *client_url);
    }

    fn update_device_to_client_info_map(
        map: &DashMap<uuid::Uuid, ClientInfo>,
        client_info: &ClientInfo,
    ) {
        map.entry(client_info.storage_token.device_id)
            .and_modify(|e| {
                if e.report_time < client_info.report_time {
                    assert_eq!(
                        e.storage_token.device_id,
                        client_info.storage_token.device_id
                    );
                    *e = client_info.clone();
                }
            })
            .or_insert(client_info.clone());
    }

    pub fn update_client(&self, stoken: StorageToken, report_time: i64) {
        let inner = self
            .0
            .org_clients_map
            .entry(stoken.organization_id.clone())
            .or_default();

        let client_info = ClientInfo {
            storage_token: stoken.clone(),
            report_time,
        };

        Self::update_device_to_client_info_map(&inner, &client_info);
    }

    pub fn remove_client(&self, stoken: &StorageToken) {
        self.0
            .org_clients_map
            .remove_if(&stoken.organization_id, |_, set| {
                Self::remove_device_to_client_info_map(set, &stoken.device_id, &stoken.client_url);
                set.is_empty()
            });
    }

    pub fn weak_ref(&self) -> WeakRefStorage {
        Arc::downgrade(&self.0)
    }

    pub fn get_client_url_by_device_id(
        &self,
        organization_id: &OrgIdInDb,
        device_id: &uuid::Uuid,
    ) -> Option<url::Url> {
        self.0
            .org_clients_map
            .get(organization_id)
            .and_then(|info_map| {
                info_map
                    .get(device_id)
                    .map(|info| info.storage_token.client_url.clone())
            })
    }

    pub fn list_organization_clients(&self, organization_id: &OrgIdInDb) -> Vec<url::Url> {
        self.0
            .org_clients_map
            .get(organization_id)
            .map(|info_map| {
                info_map
                    .iter()
                    .map(|info| info.value().storage_token.client_url.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn db(&self) -> &Database {
        &self.0.db
    }
}
