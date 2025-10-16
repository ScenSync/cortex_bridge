//! STUN wrapper module (copied from original)

use async_trait::async_trait;
use easytier::common::error::Error;
use easytier::common::stun::{MockStunInfoCollector, StunInfoCollectorTrait};
use easytier::proto::common::{NatType, StunInfo};
use std::net::SocketAddr;
use std::ops::{Deref, DerefMut};

pub struct MockStunInfoCollectorWrapper {
    inner: MockStunInfoCollector,
}

impl MockStunInfoCollectorWrapper {
    pub fn new() -> Self {
        Self {
            inner: MockStunInfoCollector {
                udp_nat_type: NatType::Unknown,
            },
        }
    }
}

impl Deref for MockStunInfoCollectorWrapper {
    type Target = MockStunInfoCollector;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for MockStunInfoCollectorWrapper {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

#[async_trait]
impl StunInfoCollectorTrait for MockStunInfoCollectorWrapper {
    fn get_stun_info(&self) -> StunInfo {
        self.inner.get_stun_info()
    }

    async fn get_udp_port_mapping(&self, local_port: u16) -> Result<SocketAddr, Error> {
        self.inner.get_udp_port_mapping(local_port).await
    }
}

impl Default for MockStunInfoCollectorWrapper {
    fn default() -> Self {
        Self::new()
    }
}
