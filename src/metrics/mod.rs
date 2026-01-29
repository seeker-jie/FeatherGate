use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// 简单的指标收集器
#[derive(Debug, Default)]
pub struct Metrics {
    total_requests: AtomicU64,
    successful_requests: AtomicU64,
    failed_requests: AtomicU64,
}

impl Metrics {
    pub fn new() -> Self {
        Self::default()
    }

    /// 记录成功请求
    pub fn record_success(&self) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.successful_requests.fetch_add(1, Ordering::Relaxed);
    }

    /// 记录失败请求
    pub fn record_failure(&self) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.failed_requests.fetch_add(1, Ordering::Relaxed);
    }

    /// 导出 Prometheus 格式
    pub fn export_prometheus(&self) -> String {
        format!(
            "# HELP feathergate_requests_total Total number of requests\n\
             # TYPE feathergate_requests_total counter\n\
             feathergate_requests_total {}\n\
             # HELP feathergate_requests_successful Successful requests\n\
             # TYPE feathergate_requests_successful counter\n\
             feathergate_requests_successful {}\n\
             # HELP feathergate_requests_failed Failed requests\n\
             # TYPE feathergate_requests_failed counter\n\
             feathergate_requests_failed {}\n",
            self.total_requests.load(Ordering::Relaxed),
            self.successful_requests.load(Ordering::Relaxed),
            self.failed_requests.load(Ordering::Relaxed)
        )
    }
}

/// 获取全局指标实例
pub fn global_metrics() -> &'static Arc<Metrics> {
    use once_cell::sync::Lazy;
    static METRICS: Lazy<Arc<Metrics>> = Lazy::new(|| Arc::new(Metrics::new()));
    &METRICS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_record() {
        let metrics = Metrics::new();

        metrics.record_success();
        metrics.record_success();
        metrics.record_failure();

        assert_eq!(metrics.total_requests.load(Ordering::Relaxed), 3);
        assert_eq!(metrics.successful_requests.load(Ordering::Relaxed), 2);
        assert_eq!(metrics.failed_requests.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_export_prometheus() {
        let metrics = Metrics::new();
        metrics.record_success();
        metrics.record_failure();

        let output = metrics.export_prometheus();
        assert!(output.contains("feathergate_requests_total 2"));
        assert!(output.contains("feathergate_requests_successful 1"));
        assert!(output.contains("feathergate_requests_failed 1"));
    }
}
