use async_trait::async_trait;

/// 查询 trait (CQRS 模式)
pub trait CqrsQuery: Send + Sync {
    type Response: Send + Sync;
}

/// 查询处理器 trait
#[async_trait]
pub trait QueryHandler<Q: CqrsQuery>: Send + Sync {
    type Error: Send + Sync;

    async fn handle(&self, query: Q) -> Result<Q::Response, Self::Error>;
}

/// 查询总线
pub trait QueryBus: Send + Sync {
    fn register_handler<H, Q>(&mut self, handler: H)
    where
        H: QueryHandler<Q> + 'static,
        Q: CqrsQuery + 'static;

    async fn ask<Q: CqrsQuery + 'static>(
        &self,
        query: Q,
    ) -> Result<Q::Response, Box<dyn std::error::Error + Send + Sync>>;
}
