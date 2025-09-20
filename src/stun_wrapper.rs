//! STUN 包装模块
//! 
//! 该模块提供了对 easytier::common::stun::MockStunInfoCollector 的包装，
//! 使用 Deref 和 DerefMut 特性实现对私有 NatType 类型的透明访问。

use std::ops::{Deref, DerefMut};
use std::net::SocketAddr;
use async_trait::async_trait;
use easytier::common::stun::{MockStunInfoCollector, StunInfoCollectorTrait};
use easytier::proto::common::{NatType, StunInfo};
use easytier::common::error::Error;

/// MockStunInfoCollectorWrapper 是对 MockStunInfoCollector 的包装
/// 通过实现 Deref 和 DerefMut 特性，允许透明访问内部的 MockStunInfoCollector
/// 同时提供了创建实例的便捷方法
pub struct MockStunInfoCollectorWrapper {
    // 内部持有一个 MockStunInfoCollector 实例
    inner: MockStunInfoCollector,
}

impl MockStunInfoCollectorWrapper {
    /// 创建一个新的 MockStunInfoCollectorWrapper 实例
    pub fn new() -> Self {
        Self {
            // 使用空结构体初始化 MockStunInfoCollector
            inner: MockStunInfoCollector {
                udp_nat_type: NatType::Unknown,
            },
        }
    }
}

// 实现 Deref 特性，允许透明地将 MockStunInfoCollectorWrapper 当作 MockStunInfoCollector 使用
impl Deref for MockStunInfoCollectorWrapper {
    type Target = MockStunInfoCollector;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

// 实现 DerefMut 特性，允许透明地将 MockStunInfoCollectorWrapper 当作可变的 MockStunInfoCollector 使用
impl DerefMut for MockStunInfoCollectorWrapper {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

// 实现 StunInfoCollectorTrait 特性，委托给内部的 MockStunInfoCollector 实例
#[async_trait]
impl StunInfoCollectorTrait for MockStunInfoCollectorWrapper {
    // 实现所需的方法，委托给内部实例
    fn get_stun_info(&self) -> StunInfo {
        self.inner.get_stun_info()
    }

    async fn get_udp_port_mapping(&self, local_port: u16) -> Result<SocketAddr, Error> {
        self.inner.get_udp_port_mapping(local_port).await
    }
}

// 实现 Default 特性，提供默认构造方法
impl Default for MockStunInfoCollectorWrapper {
    fn default() -> Self {
        Self::new()
    }
}