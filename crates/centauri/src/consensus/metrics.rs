// Phase 1.5: Advanced Metrics & Monitoring
// Production-grade prometheus-style metrics for DAG consensus

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

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

    // Gauges - current values
    active_vertices: AtomicUsize,
    pending_broadcasts: AtomicUsize,
    cache_entries: AtomicUsize,
    connected_peers: AtomicUsize,

    // Histograms - distribution tracking
    vertex_latency: RwLock<Histogram>,
    compression_ratio: RwLock<Histogram>,
    batch_size: RwLock<Histogram>,
    checkpoint_interval: RwLock<Histogram>,

    // Custom metrics
    start_time: Instant,
    custom_metrics: RwLock<HashMap<String, f64>>,
}

/// Histogram for tracking value distributions
#[derive(Debug, Clone)]
pub struct Histogram {
    values: Vec<f64>,
    sum: f64,
    count: u64,
    min: f64,
    max: f64,
    buckets: Vec<(f64, u64)>, // (upper_bound, count)
}

impl Histogram {
    fn new(buckets: Vec<f64>) -> Self {
        Self {
            values: Vec::new(),
            sum: 0.0,
            count: 0,
            min: f64::MAX,
            max: f64::MIN,
            buckets: buckets.into_iter().map(|b| (b, 0)).collect(),
        }
    }

    fn observe(&mut self, value: f64) {
        self.values.push(value);
        self.sum += value;
        self.count += 1;
        self.min = self.min.min(value);
        self.max = self.max.max(value);

        // Update buckets
        for (upper_bound, count) in &mut self.buckets {
            if value <= *upper_bound {
                *count += 1;
            }
        }
    }

    fn mean(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.sum / self.count as f64
        }
    }

    fn percentile(&self, p: f64) -> f64 {
        if self.values.is_empty() {
            return 0.0;
        }

        let mut sorted = self.values.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let index = ((p / 100.0) * sorted.len() as f64) as usize;
        sorted[index.min(sorted.len() - 1)]
    }

    #[allow(dead_code)]
    fn reset(&mut self) {
        self.values.clear();
        self.sum = 0.0;
        self.count = 0;
        self.min = f64::MAX;
        self.max = f64::MIN;
        for (_, count) in &mut self.buckets {
            *count = 0;
        }
    }

    #[allow(dead_code)]
    fn reset_allowed(&mut self) {
        // Kept as an explicit allowed duplicate to satisfy clippy when needed.
        self.reset();
    }
}

impl DagMetrics {
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

                active_vertices: AtomicUsize::new(0),
                pending_broadcasts: AtomicUsize::new(0),
                cache_entries: AtomicUsize::new(0),
                connected_peers: AtomicUsize::new(0),

                vertex_latency: RwLock::new(Histogram::new(vec![
                    0.001, 0.005, 0.010, 0.025, 0.050, 0.100, 0.250, 0.500, 1.0, 2.5, 5.0,
                ])),
                compression_ratio: RwLock::new(Histogram::new(vec![
                    0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0,
                ])),
                batch_size: RwLock::new(Histogram::new(vec![
                    10.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 2500.0, 5000.0,
                ])),
                checkpoint_interval: RwLock::new(Histogram::new(vec![
                    10.0, 50.0, 100.0, 250.0, 500.0, 1000.0,
                ])),

                start_time: Instant::now(),
                custom_metrics: RwLock::new(HashMap::new()),
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

    // ===== Histogram Operations =====

    pub fn observe_vertex_latency(&self, duration: Duration) {
        let seconds = duration.as_secs_f64();
        if let Ok(mut hist) = self.inner.vertex_latency.write() {
            hist.observe(seconds);
        }
    }

    pub fn observe_compression_ratio(&self, ratio: f64) {
        if let Ok(mut hist) = self.inner.compression_ratio.write() {
            hist.observe(ratio);
        }
    }

    pub fn observe_batch_size(&self, size: usize) {
        if let Ok(mut hist) = self.inner.batch_size.write() {
            hist.observe(size as f64);
        }
    }

    pub fn observe_checkpoint_interval(&self, vertices: u64) {
        if let Ok(mut hist) = self.inner.checkpoint_interval.write() {
            hist.observe(vertices as f64);
        }
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

        // Counters
        output.push_str("# HELP dag_vertices_created_total Total vertices created\n");
        output.push_str("# TYPE dag_vertices_created_total counter\n");
        output.push_str(&format!(
            "dag_vertices_created_total {}\n",
            self.inner.vertices_created.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP dag_vertices_broadcast_total Total vertices broadcast\n");
        output.push_str("# TYPE dag_vertices_broadcast_total counter\n");
        output.push_str(&format!(
            "dag_vertices_broadcast_total {}\n",
            self.inner.vertices_broadcast.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP dag_vertices_received_total Total vertices received\n");
        output.push_str("# TYPE dag_vertices_received_total counter\n");
        output.push_str(&format!(
            "dag_vertices_received_total {}\n",
            self.inner.vertices_received.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP dag_checkpoints_created_total Total checkpoints created\n");
        output.push_str("# TYPE dag_checkpoints_created_total counter\n");
        output.push_str(&format!(
            "dag_checkpoints_created_total {}\n",
            self.inner.checkpoints_created.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP dag_compression_operations_total Total compression operations\n");
        output.push_str("# TYPE dag_compression_operations_total counter\n");
        output.push_str(&format!(
            "dag_compression_operations_total {}\n",
            self.inner.compression_operations.load(Ordering::Relaxed)
        ));

        output
            .push_str("# HELP dag_decompression_operations_total Total decompression operations\n");
        output.push_str("# TYPE dag_decompression_operations_total counter\n");
        output.push_str(&format!(
            "dag_decompression_operations_total {}\n",
            self.inner.decompression_operations.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP dag_ecvrf_generations_total Total ECVRF generations\n");
        output.push_str("# TYPE dag_ecvrf_generations_total counter\n");
        output.push_str(&format!(
            "dag_ecvrf_generations_total {}\n",
            self.inner.ecvrf_generations.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP dag_ecvrf_verifications_total Total ECVRF verifications\n");
        output.push_str("# TYPE dag_ecvrf_verifications_total counter\n");
        output.push_str(&format!(
            "dag_ecvrf_verifications_total {}\n",
            self.inner.ecvrf_verifications.load(Ordering::Relaxed)
        ));

        // Gauges
        output.push_str("# HELP dag_active_vertices Current active vertices\n");
        output.push_str("# TYPE dag_active_vertices gauge\n");
        output.push_str(&format!(
            "dag_active_vertices {}\n",
            self.inner.active_vertices.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP dag_pending_broadcasts Current pending broadcasts\n");
        output.push_str("# TYPE dag_pending_broadcasts gauge\n");
        output.push_str(&format!(
            "dag_pending_broadcasts {}\n",
            self.inner.pending_broadcasts.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP dag_cache_entries Current cache entries\n");
        output.push_str("# TYPE dag_cache_entries gauge\n");
        output.push_str(&format!(
            "dag_cache_entries {}\n",
            self.inner.cache_entries.load(Ordering::Relaxed)
        ));

        output.push_str("# HELP dag_connected_peers Current connected peers\n");
        output.push_str("# TYPE dag_connected_peers gauge\n");
        output.push_str(&format!(
            "dag_connected_peers {}\n",
            self.inner.connected_peers.load(Ordering::Relaxed)
        ));

        // Histograms
        if let Ok(hist) = self.inner.vertex_latency.read() {
            output.push_str("# HELP dag_vertex_latency_seconds Vertex propagation latency\n");
            output.push_str("# TYPE dag_vertex_latency_seconds histogram\n");
            for (bound, count) in &hist.buckets {
                output.push_str(&format!(
                    "dag_vertex_latency_seconds_bucket{{le=\"{}\"}} {}\n",
                    bound, count
                ));
            }
            output.push_str(&format!("dag_vertex_latency_seconds_sum {}\n", hist.sum));
            output.push_str(&format!(
                "dag_vertex_latency_seconds_count {}\n",
                hist.count
            ));
        }

        if let Ok(hist) = self.inner.compression_ratio.read() {
            output
                .push_str("# HELP dag_compression_ratio Compression ratio (compressed/original)\n");
            output.push_str("# TYPE dag_compression_ratio histogram\n");
            for (bound, count) in &hist.buckets {
                output.push_str(&format!(
                    "dag_compression_ratio_bucket{{le=\"{}\"}} {}\n",
                    bound, count
                ));
            }
            output.push_str(&format!("dag_compression_ratio_sum {}\n", hist.sum));
            output.push_str(&format!("dag_compression_ratio_count {}\n", hist.count));
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
        output.push_str(&format!(
            "  Vertices Created:      {}\n",
            self.inner.vertices_created.load(Ordering::Relaxed)
        ));
        output.push_str(&format!(
            "  Vertices Broadcast:    {}\n",
            self.inner.vertices_broadcast.load(Ordering::Relaxed)
        ));
        output.push_str(&format!(
            "  Vertices Received:     {}\n",
            self.inner.vertices_received.load(Ordering::Relaxed)
        ));
        output.push_str(&format!(
            "  Checkpoints Created:   {}\n",
            self.inner.checkpoints_created.load(Ordering::Relaxed)
        ));
        output.push_str(&format!(
            "  Compression Ops:       {}\n",
            self.inner.compression_operations.load(Ordering::Relaxed)
        ));
        output.push_str(&format!(
            "  ECVRF Generations:     {}\n",
            self.inner.ecvrf_generations.load(Ordering::Relaxed)
        ));

        output.push_str("\nGauges:\n");
        output.push_str(&format!(
            "  Active Vertices:       {}\n",
            self.inner.active_vertices.load(Ordering::Relaxed)
        ));
        output.push_str(&format!(
            "  Pending Broadcasts:    {}\n",
            self.inner.pending_broadcasts.load(Ordering::Relaxed)
        ));
        output.push_str(&format!(
            "  Cache Entries:         {}\n",
            self.inner.cache_entries.load(Ordering::Relaxed)
        ));
        output.push_str(&format!(
            "  Connected Peers:       {}\n",
            self.inner.connected_peers.load(Ordering::Relaxed)
        ));

        if let Ok(hist) = self.inner.vertex_latency.read() {
            output.push_str("\nVertex Latency:\n");
            output.push_str(&format!("  Count:      {}\n", hist.count));
            output.push_str(&format!("  Mean:       {:.4}s\n", hist.mean()));
            output.push_str(&format!("  Min:        {:.4}s\n", hist.min));
            output.push_str(&format!("  Max:        {:.4}s\n", hist.max));
            output.push_str(&format!("  P50:        {:.4}s\n", hist.percentile(50.0)));
            output.push_str(&format!("  P95:        {:.4}s\n", hist.percentile(95.0)));
            output.push_str(&format!("  P99:        {:.4}s\n", hist.percentile(99.0)));
        }

        if let Ok(hist) = self.inner.compression_ratio.read() {
            output.push_str("\nCompression Ratio:\n");
            output.push_str(&format!("  Mean:       {:.2}x\n", hist.mean()));
            output.push_str(&format!("  Min:        {:.2}x\n", hist.min));
            output.push_str(&format!("  Max:        {:.2}x\n", hist.max));
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
    use std::time::Duration;

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
    fn test_histogram() {
        let metrics = DagMetrics::new();

        metrics.observe_vertex_latency(Duration::from_millis(10));
        metrics.observe_vertex_latency(Duration::from_millis(20));
        metrics.observe_vertex_latency(Duration::from_millis(30));

        let hist = metrics.inner.vertex_latency.read().unwrap();
        assert_eq!(hist.count, 3);
        assert_eq!(hist.mean(), 0.020); // 20ms average
        assert_eq!(hist.min, 0.010);
        assert_eq!(hist.max, 0.030);
    }

    #[test]
    fn test_compression_ratio() {
        let metrics = DagMetrics::new();

        metrics.observe_compression_ratio(0.5); // 50% compression
        metrics.observe_compression_ratio(0.3); // 70% compression
        metrics.observe_compression_ratio(0.4); // 60% compression

        let hist = metrics.inner.compression_ratio.read().unwrap();
        assert_eq!(hist.count, 3);
        assert!((hist.mean() - 0.4).abs() < 1e-10); // Floating point comparison with tolerance
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
