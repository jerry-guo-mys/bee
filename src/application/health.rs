//! 健康检查

/// 健康状态
#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub status: Status,
    pub checks: Vec<HealthCheck>,
}

/// 状态枚举
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    Healthy,
    Degraded,
    Unhealthy,
}

/// 健康检查项
#[derive(Debug, Clone)]
pub struct HealthCheck {
    pub name: String,
    pub status: Status,
    pub message: Option<String>,
}

/// 健康检查器
pub struct HealthChecker {
    checks: Vec<HealthCheck>,
}

impl HealthChecker {
    pub fn new() -> Self {
        Self { checks: vec![] }
    }

    /// 添加检查项
    pub fn add_check(&mut self, check: HealthCheck) {
        self.checks.push(check);
    }

    /// 执行健康检查
    pub fn check(&self) -> HealthStatus {
        let has_unhealthy = self.checks.iter().any(|c| c.status == Status::Unhealthy);
        let has_degraded = self.checks.iter().any(|c| c.status == Status::Degraded);

        let status = if has_unhealthy {
            Status::Unhealthy
        } else if has_degraded {
            Status::Degraded
        } else {
            Status::Healthy
        };

        HealthStatus {
            status,
            checks: self.checks.clone(),
        }
    }
}

impl Default for HealthChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_checker_healthy() {
        let mut checker = HealthChecker::new();
        checker.add_check(HealthCheck {
            name: "db".to_string(),
            status: Status::Healthy,
            message: None,
        });

        let status = checker.check();
        assert_eq!(status.status, Status::Healthy);
    }

    #[test]
    fn test_health_checker_unhealthy() {
        let mut checker = HealthChecker::new();
        checker.add_check(HealthCheck {
            name: "db".to_string(),
            status: Status::Unhealthy,
            message: Some("Connection failed".to_string()),
        });

        let status = checker.check();
        assert_eq!(status.status, Status::Unhealthy);
    }
}
