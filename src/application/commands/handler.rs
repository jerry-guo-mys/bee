use async_trait::async_trait;
use dashmap::DashMap;
use std::any::{Any, TypeId};
use std::error::Error;
use std::fmt;

/// 命令 trait (CQRS 模式)
pub trait CqrsCommand: Send + Sync {
    type Response: Send + Sync;
}

/// Command 别名 (简化使用)
pub trait Command: CqrsCommand {}
impl<T: CqrsCommand> Command for T {}

/// 命令处理器 trait
#[async_trait]
pub trait CommandHandler<C: CqrsCommand>: Send + Sync {
    type Error: Send + Sync;

    async fn handle(&self, command: C) -> Result<C::Response, Self::Error>;
}

/// 命令总线 trait
pub trait CommandBus: Send + Sync {
    fn register_handler<H, C>(&mut self, handler: H)
    where
        H: CommandHandler<C> + 'static,
        C: CqrsCommand + 'static;

    async fn dispatch<C: CqrsCommand + 'static>(
        &self,
        command: C,
    ) -> Result<C::Response, Box<dyn Error + Send + Sync>>;
}

/// 内存中的命令总线实现
pub struct InMemoryCommandBus {
    handlers: DashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl InMemoryCommandBus {
    pub fn new() -> Self {
        Self {
            handlers: DashMap::new(),
        }
    }
}

impl Default for InMemoryCommandBus {
    fn default() -> Self {
        Self::new()
    }
}

/// 命令总线错误类型
#[derive(Debug)]
pub struct CommandBusError(pub String);

impl fmt::Display for CommandBusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for CommandBusError {}

impl CommandBus for InMemoryCommandBus {
    fn register_handler<H, C>(&mut self, handler: H)
    where
        H: CommandHandler<C> + 'static,
        C: CqrsCommand + 'static,
    {
        self.handlers.insert(TypeId::of::<C>(), Box::new(handler));
    }

    async fn dispatch<C: CqrsCommand + 'static>(
        &self,
        command: C,
    ) -> Result<C::Response, Box<dyn Error + Send + Sync>> {
        let handler = self.handlers.get(&TypeId::of::<C>()).ok_or_else(|| {
            Box::new(CommandBusError(format!(
                "Handler not registered for command: {}",
                std::any::type_name::<C>()
            )))
        })?;

        // 类型安全的向下转型并调用 handler
        let handler = handler
            .downcast_ref::<Box<dyn CommandHandler<C, Error = anyhow::Error> + Send + Sync>>()
            .ok_or_else(|| Box::new(CommandBusError("Handler type mismatch".to_string())))?;

        // 使用 anyhow::Error 的 Debug 表示来创建错误消息
        handler.handle(command).await.map_err(|e| {
            Box::new(CommandBusError(format!("Command handler error: {}", e)))
                as Box<dyn Error + Send + Sync>
        })
    }
}
