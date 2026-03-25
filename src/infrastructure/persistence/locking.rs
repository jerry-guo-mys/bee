//! 持久化层细粒度锁实现
//!
//! 提供基于键值粒度的锁机制，支持并发安全的数据库操作

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;
use std::fmt::Debug;
use tokio::sync::{RwLock, OwnedRwLockReadGuard, OwnedRwLockWriteGuard};

/// 细粒度锁管理器
///
/// 为每个键提供独立的读写锁，实现细粒度的并发控制
pub struct FineGrainedLockStore<K, V>
where
    K: Eq + Hash + Clone + Send + Sync,
    V: Send + Sync,
{
    /// 锁映射表
    locks: Arc<RwLock<HashMap<K, Arc<RwLock<V>>>>>,
    /// 统计信息
    stats: Arc<RwLock<LockStats>>,
}

/// 锁统计信息
#[derive(Debug, Default, Clone)]
pub struct LockStats {
    /// 当前锁数量
    pub lock_count: usize,
    /// 读锁获取次数
    pub read_acquisitions: u64,
    /// 写锁获取次数
    pub write_acquisitions: u64,
    /// 锁等待次数
    pub lock_waits: u64,
}

impl<K, V> FineGrainedLockStore<K, V>
where
    K: Eq + Hash + Clone + Send + Sync + Debug + 'static,
    V: Send + Sync + Clone + 'static,
{
    /// 创建新的细粒度锁存储
    pub fn new() -> Self {
        Self {
            locks: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(LockStats::default())),
        }
    }

    /// 获取或创建锁
    async fn get_or_create_lock(&self, key: K, value: V) -> Arc<RwLock<V>> {
        // 首先尝试读锁获取
        {
            let locks_read = self.locks.read().await;
            if let Some(lock) = locks_read.get(&key) {
                return Arc::clone(lock);
            }
        }

        // 需要创建新锁，使用写锁
        let mut locks_write = self.locks.write().await;

        // 双重检查（避免竞争条件）
        if let Some(lock) = locks_write.get(&key) {
            return Arc::clone(lock);
        }

        // 创建新锁
        let new_lock = Arc::new(RwLock::new(value));
        locks_write.insert(key, Arc::clone(&new_lock));

        let mut stats = self.stats.write().await;
        stats.lock_count = locks_write.len();

        new_lock
    }

    /// 获取读锁
    pub async fn read(&self, key: K) -> Result<FineGrainedReadGuard<V>, LockError> {
        let locks_read = self.locks.read().await;

        let lock = locks_read
            .get(&key)
            .ok_or_else(|| LockError::KeyNotFound(format!("{:?}", key)))?
            .clone();

        drop(locks_read);

        {
            let mut stats = self.stats.write().await;
            stats.read_acquisitions += 1;
        }

        let lock_clone = Arc::clone(&lock);
        let guard = lock_clone.read_owned().await;
        Ok(FineGrainedReadGuard { guard, _lock: lock })
    }

    /// 获取写锁
    pub async fn write(&self, key: K) -> Result<FineGrainedWriteGuard<V>, LockError> {
        let locks_read = self.locks.read().await;

        let lock = locks_read
            .get(&key)
            .ok_or_else(|| LockError::KeyNotFound(format!("{:?}", key)))?
            .clone();

        drop(locks_read);

        {
            let mut stats = self.stats.write().await;
            stats.write_acquisitions += 1;
        }

        let lock_clone = Arc::clone(&lock);
        let guard = lock_clone.write_owned().await;
        Ok(FineGrainedWriteGuard { guard, _lock: lock })
    }

    /// 插入或更新值
    pub async fn upsert(&self, key: K, value: V) -> Result<Option<V>, LockError> {
        let lock = self.get_or_create_lock(key.clone(), value).await;
        let guard = lock.write().await;
        let old_value = guard.clone();
        Ok(Some(old_value))
    }

    /// 删除键
    pub async fn delete(&self, key: &K) -> Result<Option<V>, LockError> {
        let mut locks = self.locks.write().await;

        if let Some(lock) = locks.remove(key) {
            {
                let mut stats = self.stats.write().await;
                stats.lock_count = locks.len();
            }

            // 尝试获取写锁以提取值
            let value = match Arc::try_unwrap(lock) {
                Ok(rwlock) => Some(rwlock.into_inner()),
                Err(arc_lock) => {
                    // 还有其他引用，需要等待获取锁
                    let guard = arc_lock.write().await;
                    Some(guard.clone())
                }
            };
            return Ok(value);
        }

        Ok(None)
    }

    /// 获取所有键
    pub async fn keys(&self) -> Vec<K> {
        let locks = self.locks.read().await;
        locks.keys().cloned().collect()
    }

    /// 获取统计信息
    pub async fn stats(&self) -> LockStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// 获取锁数量
    pub async fn len(&self) -> usize {
        let locks = self.locks.read().await;
        locks.len()
    }

    /// 是否为空
    pub async fn is_empty(&self) -> bool {
        self.len().await == 0
    }
}

impl<K, V> Default for FineGrainedLockStore<K, V>
where
    K: Eq + Hash + Clone + Send + Sync + Debug + 'static,
    V: Send + Sync + Clone + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

/// 细粒度读锁守卫
pub struct FineGrainedReadGuard<V>
where
    V: Send + Sync,
{
    guard: OwnedRwLockReadGuard<V>,
    _lock: Arc<RwLock<V>>,
}

impl<V> std::ops::Deref for FineGrainedReadGuard<V>
where
    V: Send + Sync,
{
    type Target = V;

    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

/// 细粒度写锁守卫
pub struct FineGrainedWriteGuard<V>
where
    V: Send + Sync,
{
    guard: OwnedRwLockWriteGuard<V>,
    _lock: Arc<RwLock<V>>,
}

impl<V> std::ops::Deref for FineGrainedWriteGuard<V>
where
    V: Send + Sync,
{
    type Target = V;

    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

impl<V> std::ops::DerefMut for FineGrainedWriteGuard<V>
where
    V: Send + Sync,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.guard
    }
}

/// 锁错误
#[derive(Debug, thiserror::Error)]
pub enum LockError {
    #[error("Key not found: {0}")]
    KeyNotFound(String),

    #[error("Lock acquisition timeout")]
    Timeout,

    #[error("Deadlock detected")]
    Deadlock,

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// 分段锁 Map（Sharded Map）
///
/// 使用分段锁技术提高并发性能
pub struct ShardedMap<K, V>
where
    K: Eq + Hash + Clone + Send + Sync,
    V: Send + Sync,
{
    shards: Vec<Arc<RwLock<HashMap<K, V>>>>,
    shard_count: usize,
    hasher: std::collections::hash_map::RandomState,
}

impl<K, V> ShardedMap<K, V>
where
    K: Eq + Hash + Clone + Send + Sync + 'static,
    V: Send + Sync + Clone + 'static,
{
    /// 创建新的分段 Map
    pub fn new(shard_count: usize) -> Self {
        let mut shards = Vec::with_capacity(shard_count);
        for _ in 0..shard_count {
            shards.push(Arc::new(RwLock::new(HashMap::new())));
        }

        Self {
            shards,
            shard_count,
            hasher: std::collections::hash_map::RandomState::new(),
        }
    }

    /// 根据键获取分片索引
    fn shard_index(&self, key: &K) -> usize {
        use std::hash::{Hasher, BuildHasher};
        let mut h = self.hasher.build_hasher();
        key.hash(&mut h);
        h.finish() as usize % self.shard_count
    }

    /// 获取分片
    fn get_shard(&self, key: &K) -> &Arc<RwLock<HashMap<K, V>>> {
        &self.shards[self.shard_index(key)]
    }

    /// 插入键值对
    pub async fn insert(&self, key: K, value: V) -> Option<V> {
        let mut shard = self.get_shard(&key).write().await;
        shard.insert(key, value)
    }

    /// 获取值
    pub async fn get(&self, key: &K) -> Option<V> {
        let shard = self.get_shard(key).read().await;
        shard.get(key).cloned()
    }

    /// 删除键
    pub async fn remove(&self, key: &K) -> Option<V> {
        let mut shard = self.get_shard(key).write().await;
        shard.remove(key)
    }

    /// 检查键是否存在
    pub async fn contains_key(&self, key: &K) -> bool {
        let shard = self.get_shard(key).read().await;
        shard.contains_key(key)
    }

    /// 获取所有键
    pub async fn keys(&self) -> Vec<K> {
        let mut keys = Vec::new();
        for shard in &self.shards {
            let shard_read = shard.read().await;
            keys.extend(shard_read.keys().cloned());
        }
        keys
    }

    /// 获取大小
    pub async fn len(&self) -> usize {
        let mut total = 0;
        for shard in &self.shards {
            let shard_read = shard.read().await;
            total += shard_read.len();
        }
        total
    }

    /// 是否为空
    pub async fn is_empty(&self) -> bool {
        self.len().await == 0
    }

    /// 清除所有数据
    pub async fn clear(&self) {
        for shard in &self.shards {
            let mut shard_write = shard.write().await;
            shard_write.clear();
        }
    }
}

impl<K, V> Default for ShardedMap<K, V>
where
    K: Eq + Hash + Clone + Send + Sync + 'static,
    V: Send + Sync + Clone + 'static,
{
    fn default() -> Self {
        Self::new(16)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fine_grained_lock_basic() {
        let store: FineGrainedLockStore<String, i32> = FineGrainedLockStore::new();

        // 初始化数据
        store.upsert("key1".to_string(), 100).await.unwrap();
        store.upsert("key2".to_string(), 200).await.unwrap();

        // 读测试
        let guard = store.read("key1".to_string()).await.unwrap();
        assert_eq!(*guard, 100);
        drop(guard);

        // 写测试
        let mut guard = store.write("key1".to_string()).await.unwrap();
        *guard = 150;
        drop(guard);

        // 验证更新
        let guard = store.read("key1".to_string()).await.unwrap();
        assert_eq!(*guard, 150);
    }

    #[tokio::test]
    async fn test_sharded_map_basic() {
        let map: ShardedMap<String, i32> = ShardedMap::new(4);

        map.insert("key1".to_string(), 100).await;
        map.insert("key2".to_string(), 200).await;

        assert_eq!(map.get(&"key1".to_string()).await, Some(100));
        assert_eq!(map.get(&"key2".to_string()).await, Some(200));
        assert!(map.contains_key(&"key1".to_string()).await);

        map.remove(&"key1".to_string()).await;
        assert!(!map.contains_key(&"key1".to_string()).await);
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        let store: Arc<FineGrainedLockStore<u64, String>> = Arc::new(FineGrainedLockStore::new());
        let mut handles = Vec::new();

        // 并发写入
        for i in 0..10 {
            let store_clone = Arc::clone(&store);
            let handle = tokio::spawn(async move {
                store_clone.upsert(i, format!("value_{}", i)).await.unwrap();
            });
            handles.push(handle);
        }

        // 并发读取
        for i in 0..10 {
            let store_clone = Arc::clone(&store);
            let handle = tokio::spawn(async move {
                let guard = store_clone.read(i).await.unwrap();
                assert_eq!(guard.as_str(), format!("value_{}", i));
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }
    }
}
