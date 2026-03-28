use async_trait::async_trait;
use dashmap::DashMap;
use std::any::{Any, TypeId};
use std::error::Error;
use std::fmt;

/// 查询 trait (CQRS 模式)
pub trait CqrsQuery: Send + Sync {
    type Response: Send + Sync;
}

/// Query 别名 (简化使用)
pub trait Query: CqrsQuery {}
impl<T: CqrsQuery> Query for T {}

/// 查询处理器 trait
#[async_trait]
pub trait QueryHandler<Q: CqrsQuery>: Send + Sync {
    type Error: Send + Sync;

    async fn handle(&self, query: Q) -> Result<Q::Response, Self::Error>;
}

/// 查询总线 trait
pub trait QueryBus: Send + Sync {
    fn register_handler<H, Q>(&mut self, handler: H)
    where
        H: QueryHandler<Q> + 'static,
        Q: CqrsQuery + 'static;

    async fn ask<Q: CqrsQuery + 'static>(
        &self,
        query: Q,
    ) -> Result<Q::Response, Box<dyn Error + Send + Sync>>;
}

/// 内存中的查询总线实现
pub struct InMemoryQueryBus {
    handlers: DashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl InMemoryQueryBus {
    pub fn new() -> Self {
        Self {
            handlers: DashMap::new(),
        }
    }
}

impl Default for InMemoryQueryBus {
    fn default() -> Self {
        Self::new()
    }
}

/// 查询总线错误类型
#[derive(Debug)]
pub struct QueryBusError(pub String);

impl fmt::Display for QueryBusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for QueryBusError {}

impl QueryBus for InMemoryQueryBus {
    fn register_handler<H, Q>(&mut self, handler: H)
    where
        H: QueryHandler<Q> + 'static,
        Q: CqrsQuery + 'static,
    {
        self.handlers.insert(TypeId::of::<Q>(), Box::new(handler));
    }

    async fn ask<Q: CqrsQuery + 'static>(
        &self,
        query: Q,
    ) -> Result<Q::Response, Box<dyn Error + Send + Sync>> {
        let handler = self
            .handlers
            .get(&TypeId::of::<Q>())
            .ok_or_else(|| Box::new(QueryBusError(format!("Handler not registered for query: {}", std::any::type_name::<Q>()))))?;

        // 类型安全的向下转型并调用 handler
        let handler = handler
            .downcast_ref::<Box<dyn QueryHandler<Q, Error = anyhow::Error> + Send + Sync>>()
            .ok_or_else(|| Box::new(QueryBusError("Handler type mismatch".to_string())))?;

        // 使用 anyhow::Error 的 Debug 表示来创建错误消息
        handler.handle(query).await.map_err(|e| {
            Box::new(QueryBusError(format!("Query handler error: {}", e))) as Box<dyn Error + Send + Sync>
        })
    }
}
