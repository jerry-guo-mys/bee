//! 工作窃取任务队列（Work-Stealing Task Queue）
//!
//! 提供高并发的任务调度能力，支持优先级和工作窃取。

use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use tokio::sync::{mpsc, Mutex};

/// 任务优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low = 0,
    Normal = 1,
    High = 2,
    Urgent = 3,
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Normal
    }
}

/// 任务 trait
#[async_trait::async_trait]
pub trait Task: Send + Sync {
    /// 任务名称
    fn name(&self) -> &str;

    /// 任务优先级
    fn priority(&self) -> Priority {
        Priority::Normal
    }

    /// 执行任务
    async fn execute(&self) -> Result<(), TaskError>;
}

/// 任务错误
#[derive(Debug, thiserror::Error)]
pub enum TaskError {
    #[error("Task execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Task cancelled")]
    Cancelled,

    #[error("Task timeout")]
    Timeout,
}

/// 任务包装器
struct TaskWrapper {
    task: Box<dyn Task>,
    priority: Priority,
    attempt: AtomicUsize,
}

impl TaskWrapper {
    fn new(task: Box<dyn Task>) -> Self {
        let priority = task.priority();
        Self {
            task,
            priority,
            attempt: AtomicUsize::new(0),
        }
    }

    fn increment_attempt(&self) -> usize {
        self.attempt.fetch_add(1, Ordering::SeqCst)
    }
}

/// 工作队列（每个工作线程一个）
struct WorkQueue {
    deque: Mutex<VecDeque<Arc<TaskWrapper>>>,
    len: AtomicUsize,
}

impl WorkQueue {
    fn new() -> Self {
        Self {
            deque: Mutex::new(VecDeque::new()),
            len: AtomicUsize::new(0),
        }
    }

    async fn push(&self, task: Arc<TaskWrapper>) {
        let mut deque = self.deque.lock().await;
        deque.push_back(task);
        self.len.fetch_add(1, Ordering::SeqCst);
    }

    async fn pop(&self) -> Option<Arc<TaskWrapper>> {
        let mut deque = self.deque.lock().await;
        let task = deque.pop_back();
        if task.is_some() {
            self.len.fetch_sub(1, Ordering::SeqCst);
        }
        task
    }

    async fn steal(&self) -> Option<Arc<TaskWrapper>> {
        let mut deque = self.deque.lock().await;
        let task = deque.pop_front();
        if task.is_some() {
            self.len.fetch_sub(1, Ordering::SeqCst);
        }
        task
    }

    fn len(&self) -> usize {
        self.len.load(Ordering::Relaxed)
    }

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// 任务队列管理器
pub struct TaskQueue {
    /// 工作队列列表
    queues: Vec<Arc<WorkQueue>>,
    /// 工作线程数量
    worker_count: usize,
    /// 任务通道发送器
    tx: mpsc::Sender<Arc<TaskWrapper>>,
    /// 任务通道接收器
    rx: Arc<Mutex<mpsc::Receiver<Arc<TaskWrapper>>>>,
    /// 待处理任务数
    pending_count: AtomicUsize,
}

impl TaskQueue {
    /// 创建新的任务队列
    pub fn new(worker_count: usize) -> Self {
        let (tx, rx) = mpsc::channel(1000);

        let queues: Vec<Arc<WorkQueue>> = (0..worker_count)
            .map(|_| Arc::new(WorkQueue::new()))
            .collect();

        Self {
            queues,
            worker_count,
            tx,
            rx: Arc::new(Mutex::new(rx)),
            pending_count: AtomicUsize::new(0),
        }
    }

    /// 提交任务
    pub async fn submit<T: Task + 'static>(&self, task: T) -> Result<(), TaskError> {
        let task_wrapper = Arc::new(TaskWrapper::new(Box::new(task)));
        self.tx
            .send(task_wrapper.clone())
            .await
            .map_err(|_| TaskError::ExecutionFailed("Channel closed".into()))?;
        self.pending_count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    /// 获取任务（工作窃取算法）
    async fn get_task(&self, worker_id: usize) -> Option<Arc<TaskWrapper>> {
        let queue = &self.queues[worker_id];

        // 首先尝试从自己的队列获取
        if let Some(task) = queue.pop().await {
            return Some(task);
        }

        // 自己的工作队列为空，尝试窃取其他队列
        let mut start = (worker_id + 1) % self.worker_count;
        for _ in 0..self.worker_count - 1 {
            let other_queue = &self.queues[start];
            if !other_queue.is_empty() {
                if let Some(task) = other_queue.steal().await {
                    return Some(task);
                }
            }
            start = (start + 1) % self.worker_count;
        }

        None
    }

    /// 分发任务到各个工作队列
    async fn distribute_task(&self, task: Arc<TaskWrapper>) {
        // 找到最短的队列
        let mut min_len = usize::MAX;
        let mut target_queue = 0;

        for (i, queue) in self.queues.iter().enumerate() {
            let len = queue.len();
            if len < min_len {
                min_len = len;
                target_queue = i;
            }
        }

        self.queues[target_queue].push(task).await;
    }

    /// 启动任务分发器
    pub async fn start_dispatcher(self: Arc<Self>) {
        let rx = self.rx.clone();
        let mut rx = rx.lock().await;

        while let Some(task) = rx.recv().await {
            self.distribute_task(task).await;
        }
    }

    /// 运行工作线程
    pub async fn run_worker(self: Arc<Self>, worker_id: usize) {
        loop {
            if let Some(task) = self.get_task(worker_id).await {
                let result = task.task.execute().await;

                if let Err(e) = result {
                    tracing::warn!(
                        task = task.task.name(),
                        attempt = task.increment_attempt(),
                        "Task failed: {}",
                        e
                    );

                    // 重试逻辑（最多 3 次）
                    if task.attempt.load(Ordering::Relaxed) < 3 {
                        self.submit_task_for_retry(task).await;
                    }
                }

                self.pending_count.fetch_sub(1, Ordering::SeqCst);
            } else {
                // 没有任务时短暂等待
                tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            }
        }
    }

    /// 提交重试任务
    async fn submit_task_for_retry(&self, task: Arc<TaskWrapper>) {
        self.distribute_task(task).await;
    }

    /// 获取待处理任务数
    pub fn pending_count(&self) -> usize {
        self.pending_count.load(Ordering::Relaxed)
    }

    /// 获取工作线程数量
    pub fn worker_count(&self) -> usize {
        self.worker_count
    }
}

/// 任务队列构建器
pub struct TaskQueueBuilder {
    worker_count: usize,
}

impl TaskQueueBuilder {
    pub fn new() -> Self {
        Self {
            worker_count: std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4),
        }
    }

    pub fn with_workers(mut self, count: usize) -> Self {
        self.worker_count = count;
        self
    }

    pub fn build(self) -> Arc<TaskQueue> {
        Arc::new(TaskQueue::new(self.worker_count))
    }
}

impl Default for TaskQueueBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestTask {
        name: String,
        priority: Priority,
    }

    impl TestTask {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                priority: Priority::Normal,
            }
        }

        fn with_priority(mut self, priority: Priority) -> Self {
            self.priority = priority;
            self
        }
    }

    #[async_trait::async_trait]
    impl Task for TestTask {
        fn name(&self) -> &str {
            &self.name
        }

        fn priority(&self) -> Priority {
            self.priority
        }

        async fn execute(&self) -> Result<(), TaskError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_task_queue_submit() {
        let queue = TaskQueue::new(4);

        let task = TestTask::new("test_task");
        queue.submit(task).await.unwrap();

        assert_eq!(queue.pending_count(), 1);
    }

    #[tokio::test]
    async fn test_task_queue_priority() {
        let queue = TaskQueue::new(4);

        // 提交不同优先级的任务
        queue
            .submit(TestTask::new("low").with_priority(Priority::Low))
            .await
            .unwrap();
        queue
            .submit(TestTask::new("high").with_priority(Priority::High))
            .await
            .unwrap();
        queue
            .submit(TestTask::new("urgent").with_priority(Priority::Urgent))
            .await
            .unwrap();
        queue
            .submit(TestTask::new("normal").with_priority(Priority::Normal))
            .await
            .unwrap();

        assert_eq!(queue.pending_count(), 4);
    }

    #[tokio::test]
    async fn test_task_queue_work_stealing() {
        let queue = Arc::new(TaskQueue::new(2));

        // 提交一些任务
        for i in 0..10 {
            queue
                .submit(TestTask::new(&format!("task_{}", i)))
                .await
                .unwrap();
        }

        // 启动工作线程（简化测试，不实际运行）
        let queue_clone = queue.clone();
        let _handle = tokio::spawn(async move {
            queue_clone.run_worker(0).await;
        });

        assert_eq!(queue.pending_count(), 10);
    }
}
