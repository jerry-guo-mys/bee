//! 依赖注入容器
//!
//! 提供类型安全的组件容器，用于管理应用生命周期和依赖关系

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

/// 依赖容器
pub struct Container {
    components: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl Default for Container {
    fn default() -> Self {
        Self::new()
    }
}

impl Container {
    /// 创建新容器
    pub fn new() -> Self {
        Self {
            components: HashMap::new(),
        }
    }

    /// 注册组件
    pub fn register<T: 'static + Send + Sync>(&mut self, component: T) {
        self.components.insert(TypeId::of::<T>(), Box::new(component));
    }

    /// 注册 Arc 组件
    pub fn register_arc<T: 'static + Send + Sync>(&mut self, component: Arc<T>) {
        self.components.insert(TypeId::of::<Arc<T>>(), Box::new(component));
    }

    /// 获取组件引用
    pub fn get<T: 'static + Send + Sync>(&self) -> Option<&T> {
        self.components
            .get(&TypeId::of::<T>())
            .and_then(|b| b.downcast_ref())
    }

    /// 获取 Arc 组件
    pub fn get_arc<T: 'static + Send + Sync>(&self) -> Option<Arc<T>> {
        self.components
            .get(&TypeId::of::<Arc<T>>())
            .and_then(|b| b.downcast_ref::<Arc<T>>().cloned())
    }

    /// 检查组件是否已注册
    pub fn has<T: 'static + Send + Sync>(&self) -> bool {
        self.components.contains_key(&TypeId::of::<T>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_register_and_get() {
        let mut container = Container::new();
        
        #[derive(Debug, PartialEq)]
        struct TestComponent(String);
        
        container.register(TestComponent("test".to_string()));
        
        let component = container.get::<TestComponent>().unwrap();
        assert_eq!(component.0, "test");
    }
}
