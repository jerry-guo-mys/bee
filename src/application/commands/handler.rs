use async_trait::async_trait;

/// 命令 trait (CQRS 模式)
pub trait CqrsCommand: Send + Sync {
    type Response: Send + Sync;
}

/// 命令处理器 trait
#[async_trait]
pub trait CommandHandler<C: CqrsCommand>: Send + Sync {
    type Error: Send + Sync;

    async fn handle(&self, command: C) -> Result<C::Response, Self::Error>;
}

/// 命令总线
pub trait CommandBus: Send + Sync {
    fn register_handler<H, C>(&mut self, handler: H)
    where
        H: CommandHandler<C> + 'static,
        C: CqrsCommand + 'static;

    async fn dispatch<C: CqrsCommand + 'static>(
        &self,
        command: C,
    ) -> Result<C::Response, Box<dyn std::error::Error + Send + Sync>>;
}
