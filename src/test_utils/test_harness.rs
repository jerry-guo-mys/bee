//! 测试夹具

/// 测试夹具
pub struct TestHarness;

impl TestHarness {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TestHarness {
    fn default() -> Self {
        Self::new()
    }
}
