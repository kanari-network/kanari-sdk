// Phase 1.5: Advanced Metrics & Monitoring
// Production-grade prometheus-style metrics for DAG consensus

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Instant;

/// Thread-safe metrics collector for DAG consensus
#[derive(Clone)]
pub struct DagMetrics {
    inner: Arc<DagMetricsInner>,
}

struct DagMetricsInner {
    // Counters - monotonically increasing
    vertices_created: AtomicU64,
    vertices_broadcast: AtomicU64,
    vertices_received: AtomicU64,
    checkpoints_created: AtomicU64,
    compression_operations: AtomicU64,
    decompression_operations: AtomicU64,
    ecvrf_generations: AtomicU64,
    ecvrf_verifications: AtomicU64,
    // FIX #10: Track disk I/O queue saturation for monitoring
    disk_queue_full_count: AtomicU64,

    // Gauges - current values
    active_vertices: AtomicUsize,
    pending_broadcasts: AtomicUsize,
    cache_entries: AtomicUsize,
    connected_peers: AtomicUsize,

    // Custom metrics
    start_time: Instant,
    custom_metrics: RwLock<BTreeMap<String, f64>>,
}

impl DagMetrics {
    fn counter(counter: &AtomicU64) -> u64 {
        counter.load(Ordering::Relaxed)
    }

    fn gauge(gauge: &AtomicUsize) -> usize {
        gauge.load(Ordering::Relaxed)
    }

    fn write_counter_metric(output: &mut String, name: &str, help: &str, value: u64) {
        output.push_str(&format!("# HELP {} {}\n", name, help));
        output.push_str(&format!("# TYPE {} counter\n", name));
        output.push_str(&format!("{} {}\n", name, value));
    }

    fn write_gauge_metric(output: &mut String, name: &str, help: &str, value: usize) {
        output.push_str(&format!("# HELP {} {}\n", name, help));
        output.push_str(&format!("# TYPE {} gauge\n", name));
        output.push_str(&format!("{} {}\n", name, value));
    }

    /// Create new metrics collector
    pub fn new() -> Self {
        Self {
            inner: Arc::new(DagMetricsInner {
                vertices_created: AtomicU64::new(0),
                vertices_broadcast: AtomicU64::new(0),
                vertices_received: AtomicU64::new(0),
                checkpoints_created: AtomicU64::new(0),
                compression_operations: AtomicU64::new(0),
                decompression_operations: AtomicU64::new(0),
                ecvrf_generations: AtomicU64::new(0),
                ecvrf_verifications: AtomicU64::new(0),
                disk_queue_full_count: AtomicU64::new(0),

                active_vertices: AtomicUsize::new(0),
                pending_broadcasts: AtomicUsize::new(0),
                cache_entries: AtomicUsize::new(0),
                connected_peers: AtomicUsize::new(0),

                start_time: Instant::now(),
                custom_metrics: RwLock::new(BTreeMap::new()),
            }),
        }
    }

    // ===== Counter Operations =====

    pub fn inc_vertices_created(&self) {
        self.inner.vertices_created.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_vertices_broadcast(&self) {
        self.inner
            .vertices_broadcast
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_vertices_received(&self) {
        self.inner.vertices_received.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_checkpoints_created(&self) {
        self.inner
            .checkpoints_created
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_compression_operations(&self) {
        self.inner
            .compression_operations
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_decompression_operations(&self) {
        self.inner
            .decompression_operations
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_ecvrf_generations(&self) {
        self.inner.ecvrf_generations.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_ecvrf_verifications(&self) {
        self.inner
            .ecvrf_verifications
            .fetch_add(1, Ordering::Relaxed);
    }

    // FIX #10: Increment disk queue full counter for monitoring
    pub fn inc_disk_queue_full_count(&self) {
        self.inner
            .disk_queue_full_count
            .fetch_add(1, Ordering::Relaxed);
    }

    // ===== Gauge Operations =====

    pub fn set_active_vertices(&self, count: usize) {
        self.inner.active_vertices.store(count, Ordering::Relaxed);
    }

    pub fn set_pending_broadcasts(&self, count: usize) {
        self.inner
            .pending_broadcasts
            .store(count, Ordering::Relaxed);
    }

    pub fn set_cache_entries(&self, count: usize) {
        self.inner.cache_entries.store(count, Ordering::Relaxed);
    }

    pub fn set_connected_peers(&self, count: usize) {
        self.inner.connected_peers.store(count, Ordering::Relaxed);
    }

    // ===== Custom Metrics =====

    pub fn set_custom_metric(&self, name: String, value: f64) {
        if let Ok(mut metrics) = self.inner.custom_metrics.write() {
            metrics.insert(name, value);
        }
    }

    pub fn get_custom_metric(&self, name: &str) -> Option<f64> {
        self.inner.custom_metrics.read().ok()?.get(name).copied()
    }

    // ===== Prometheus-style Export =====

    pub fn export_prometheus(&self) -> String {
        let mut output = String::new();

        let counters = [
            (
                "dag_vertices_created_total",
                "Total vertices created",
                Self::counter(&self.inner.vertices_created),
            ),
            (
                "dag_vertices_broadcast_total",
                "Total vertices broadcast",
                Self::counter(&self.inner.vertices_broadcast),
            ),
            (
                "dag_vertices_received_total",
                "Total vertices received",
                Self::counter(&self.inner.vertices_received),
            ),
            (
                "dag_checkpoints_created_total",
                "Total checkpoints created",
                Self::counter(&self.inner.checkpoints_created),
            ),
            (
                "dag_compression_operations_total",
                "Total compression operations",
                Self::counter(&self.inner.compression_operations),
            ),
            (
                "dag_decompression_operations_total",
                "Total decompression operations",
                Self::counter(&self.inner.decompression_operations),
            ),
            (
                "dag_ecvrf_generations_total",
                "Total ECVRF generations",
                Self::counter(&self.inner.ecvrf_generations),
            ),
            (
                "dag_ecvrf_verifications_total",
                "Total ECVRF verifications",
                Self::counter(&self.inner.ecvrf_verifications),
            ),
            // FIX #10: Export disk queue saturation metric for monitoring
            (
                "dag_disk_queue_full_total",
                "Total times disk write queue was full (backpressure events)",
                Self::counter(&self.inner.disk_queue_full_count),
            ),
        ];
        for (name, help, value) in counters {
            Self::write_counter_metric(&mut output, name, help, value);
        }

        let gauges = [
            (
                "dag_active_vertices",
                "Current active vertices",
                Self::gauge(&self.inner.active_vertices),
            ),
            (
                "dag_pending_broadcasts",
                "Current pending broadcasts",
                Self::gauge(&self.inner.pending_broadcasts),
            ),
            (
                "dag_cache_entries",
                "Current cache entries",
                Self::gauge(&self.inner.cache_entries),
            ),
            (
                "dag_connected_peers",
                "Current connected peers",
                Self::gauge(&self.inner.connected_peers),
            ),
        ];
        for (name, help, value) in gauges {
            Self::write_gauge_metric(&mut output, name, help, value);
        }

        // System info
        output.push_str("# HELP dag_uptime_seconds System uptime\n");
        output.push_str("# TYPE dag_uptime_seconds gauge\n");
        output.push_str(&format!(
            "dag_uptime_seconds {}\n",
            self.inner.start_time.elapsed().as_secs_f64()
        ));

        output
    }

    // ===== Human-readable Summary =====

    pub fn summary(&self) -> String {
        let mut output = String::new();

        output.push_str("=== DAG Consensus Metrics ===\n\n");

        output.push_str("Counters:\n");
        let counter_lines = [
            (
                "Vertices Created",
                Self::counter(&self.inner.vertices_created),
            ),
            (
                "Vertices Broadcast",
                Self::counter(&self.inner.vertices_broadcast),
            ),
            (
                "Vertices Received",
                Self::counter(&self.inner.vertices_received),
            ),
            (
                "Checkpoints Created",
                Self::counter(&self.inner.checkpoints_created),
            ),
            (
                "Compression Ops",
                Self::counter(&self.inner.compression_operations),
            ),
            (
                "ECVRF Generations",
                Self::counter(&self.inner.ecvrf_generations),
            ),
        ];
        for (label, value) in counter_lines {
            output.push_str(&format!("  {:<22} {}\n", format!("{}:", label), value));
        }

        output.push_str("\nGauges:\n");
        let gauge_lines = [
            ("Active Vertices", Self::gauge(&self.inner.active_vertices)),
            (
                "Pending Broadcasts",
                Self::gauge(&self.inner.pending_broadcasts),
            ),
            ("Cache Entries", Self::gauge(&self.inner.cache_entries)),
            ("Connected Peers", Self::gauge(&self.inner.connected_peers)),
        ];
        for (label, value) in gauge_lines {
            output.push_str(&format!("  {:<22} {}\n", format!("{}:", label), value));
        }

        output.push_str(&format!(
            "\nUptime: {:.2}s\n",
            self.inner.start_time.elapsed().as_secs_f64()
        ));

        output
    }
}

impl Default for DagMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_counters() {
        let metrics = DagMetrics::new();

        assert_eq!(metrics.inner.vertices_created.load(Ordering::Relaxed), 0);

        metrics.inc_vertices_created();
        metrics.inc_vertices_created();
        metrics.inc_vertices_broadcast();

        assert_eq!(metrics.inner.vertices_created.load(Ordering::Relaxed), 2);
        assert_eq!(metrics.inner.vertices_broadcast.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_gauges() {
        let metrics = DagMetrics::new();

        metrics.set_active_vertices(42);
        metrics.set_connected_peers(5);

        assert_eq!(metrics.inner.active_vertices.load(Ordering::Relaxed), 42);
        assert_eq!(metrics.inner.connected_peers.load(Ordering::Relaxed), 5);

        metrics.set_active_vertices(100);
        assert_eq!(metrics.inner.active_vertices.load(Ordering::Relaxed), 100);
    }

    #[test]
    fn test_custom_metrics() {
        let metrics = DagMetrics::new();

        metrics.set_custom_metric("custom_value".to_string(), 42.5);

        assert_eq!(metrics.get_custom_metric("custom_value"), Some(42.5));
        assert_eq!(metrics.get_custom_metric("nonexistent"), None);
    }

    #[test]
    fn test_prometheus_export() {
        let metrics = DagMetrics::new();

        metrics.inc_vertices_created();
        metrics.inc_vertices_created();
        metrics.set_active_vertices(10);

        let export = metrics.export_prometheus();

        assert!(export.contains("dag_vertices_created_total 2"));
        assert!(export.contains("dag_active_vertices 10"));
        assert!(export.contains("# HELP"));
        assert!(export.contains("# TYPE"));
    }

    #[test]
    fn test_summary() {
        let metrics = DagMetrics::new();

        metrics.inc_vertices_created();
        metrics.inc_checkpoints_created();
        metrics.set_active_vertices(5);

        let summary = metrics.summary();

        assert!(summary.contains("Vertices Created:      1"));
        assert!(summary.contains("Checkpoints Created:   1"));
        assert!(summary.contains("Active Vertices:       5"));
    }

    #[test]
    fn test_thread_safety() {
        let metrics = DagMetrics::new();
        let metrics_clone = metrics.clone();

        let handle = thread::spawn(move || {
            for _ in 0..100 {
                metrics_clone.inc_vertices_created();
            }
        });

        for _ in 0..100 {
            metrics.inc_vertices_created();
        }

        handle.join().unwrap();

        assert_eq!(metrics.inner.vertices_created.load(Ordering::Relaxed), 200);
    }
}
