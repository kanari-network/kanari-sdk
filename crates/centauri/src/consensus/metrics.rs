// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

// DAG consensus metrics primitives used by the runtime and tests.
use anyhow::{Context, Result};
use prometheus::{Encoder, IntCounter, IntGauge, Opts, Registry, TextEncoder};
#[cfg(test)]
use std::collections::BTreeMap;
use std::sync::Arc;
#[cfg(test)]
use std::sync::RwLock;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Instant;

/// Immutable snapshot of DAG metrics at a point in time.
#[derive(Debug, Clone, PartialEq)]
pub struct DagMetricsSnapshot {
    pub vertices_created: u64,
    pub vertices_broadcast: u64,
    pub checkpoints_created: u64,
    pub disk_queue_full_count: u64,
    pub active_vertices: usize,
    pub uptime_seconds: f64,
}

/// Thread-safe metrics collector for DAG consensus
#[derive(Clone)]
pub struct DagMetrics {
    inner: Arc<DagMetricsInner>,
}

struct DagMetricsInner {
    // Counters - monotonically increasing
    vertices_created: AtomicU64,
    vertices_broadcast: AtomicU64,
    #[cfg(test)]
    vertices_received: AtomicU64,
    checkpoints_created: AtomicU64,
    #[cfg(test)]
    compression_operations: AtomicU64,
    #[cfg(test)]
    ecvrf_generations: AtomicU64,
    disk_queue_full_count: AtomicU64,

    // Gauges - current values
    active_vertices: AtomicUsize,
    #[cfg(test)]
    pending_broadcasts: AtomicUsize,
    #[cfg(test)]
    cache_entries: AtomicUsize,
    #[cfg(test)]
    connected_peers: AtomicUsize,

    start_time: Instant,
    #[cfg(test)]
    custom_metrics: RwLock<BTreeMap<String, f64>>,
}

impl DagMetrics {
    #[cfg(test)]
    fn counter(counter: &AtomicU64) -> u64 {
        counter.load(Ordering::Relaxed)
    }

    #[cfg(test)]
    fn gauge(gauge: &AtomicUsize) -> usize {
        gauge.load(Ordering::Relaxed)
    }

    /// Create new metrics collector
    pub fn new() -> Self {
        Self {
            inner: Arc::new(DagMetricsInner {
                vertices_created: AtomicU64::new(0),
                vertices_broadcast: AtomicU64::new(0),
                #[cfg(test)]
                vertices_received: AtomicU64::new(0),
                checkpoints_created: AtomicU64::new(0),
                #[cfg(test)]
                compression_operations: AtomicU64::new(0),
                #[cfg(test)]
                ecvrf_generations: AtomicU64::new(0),
                disk_queue_full_count: AtomicU64::new(0),

                active_vertices: AtomicUsize::new(0),
                #[cfg(test)]
                pending_broadcasts: AtomicUsize::new(0),
                #[cfg(test)]
                cache_entries: AtomicUsize::new(0),
                #[cfg(test)]
                connected_peers: AtomicUsize::new(0),

                start_time: Instant::now(),
                #[cfg(test)]
                custom_metrics: RwLock::new(BTreeMap::new()),
            }),
        }
    }

    pub fn snapshot(&self) -> DagMetricsSnapshot {
        DagMetricsSnapshot {
            vertices_created: self.inner.vertices_created.load(Ordering::Relaxed),
            vertices_broadcast: self.inner.vertices_broadcast.load(Ordering::Relaxed),
            checkpoints_created: self.inner.checkpoints_created.load(Ordering::Relaxed),
            disk_queue_full_count: self.inner.disk_queue_full_count.load(Ordering::Relaxed),
            active_vertices: self.inner.active_vertices.load(Ordering::Relaxed),
            uptime_seconds: self.inner.start_time.elapsed().as_secs_f64(),
        }
    }

    pub fn export_prometheus(&self) -> Result<String> {
        let snapshot = self.snapshot();
        let registry = Registry::new();

        let vertices_created = IntCounter::with_opts(Opts::new(
            "dag_vertices_created_total",
            "Total vertices created",
        ))
        .context("failed to create dag_vertices_created_total metric")?;
        vertices_created.inc_by(snapshot.vertices_created);
        registry
            .register(Box::new(vertices_created))
            .context("failed to register dag_vertices_created_total")?;

        let vertices_broadcast = IntCounter::with_opts(Opts::new(
            "dag_vertices_broadcast_total",
            "Total vertices broadcast",
        ))
        .context("failed to create dag_vertices_broadcast_total metric")?;
        vertices_broadcast.inc_by(snapshot.vertices_broadcast);
        registry
            .register(Box::new(vertices_broadcast))
            .context("failed to register dag_vertices_broadcast_total")?;

        let checkpoints_created = IntCounter::with_opts(Opts::new(
            "dag_checkpoints_created_total",
            "Total checkpoints created",
        ))
        .context("failed to create dag_checkpoints_created_total metric")?;
        checkpoints_created.inc_by(snapshot.checkpoints_created);
        registry
            .register(Box::new(checkpoints_created))
            .context("failed to register dag_checkpoints_created_total")?;

        let disk_queue_full = IntCounter::with_opts(Opts::new(
            "dag_disk_queue_full_total",
            "Total times the DAG disk write queue reported backpressure",
        ))
        .context("failed to create dag_disk_queue_full_total metric")?;
        disk_queue_full.inc_by(snapshot.disk_queue_full_count);
        registry
            .register(Box::new(disk_queue_full))
            .context("failed to register dag_disk_queue_full_total")?;

        let active_vertices = IntGauge::with_opts(Opts::new(
            "dag_active_vertices",
            "Current number of active DAG vertices tracked in memory",
        ))
        .context("failed to create dag_active_vertices metric")?;
        active_vertices.set(snapshot.active_vertices as i64);
        registry
            .register(Box::new(active_vertices))
            .context("failed to register dag_active_vertices")?;

        let uptime_seconds = prometheus::Gauge::with_opts(Opts::new(
            "dag_uptime_seconds",
            "DAG metrics collector uptime in seconds",
        ))
        .context("failed to create dag_uptime_seconds metric")?;
        uptime_seconds.set(snapshot.uptime_seconds);
        registry
            .register(Box::new(uptime_seconds))
            .context("failed to register dag_uptime_seconds")?;

        let metric_families = registry.gather();
        let encoder = TextEncoder::new();
        let mut buffer = Vec::new();
        encoder
            .encode(&metric_families, &mut buffer)
            .context("failed to encode prometheus metrics")?;

        String::from_utf8(buffer).context("failed to build utf-8 prometheus payload")
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

    pub fn inc_checkpoints_created(&self) {
        self.inner
            .checkpoints_created
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_disk_queue_full_count(&self) {
        self.inner
            .disk_queue_full_count
            .fetch_add(1, Ordering::Relaxed);
    }

    // ===== Gauge Operations =====

    pub fn set_active_vertices(&self, count: usize) {
        self.inner.active_vertices.store(count, Ordering::Relaxed);
    }

    #[cfg(test)]
    pub fn set_connected_peers(&self, count: usize) {
        self.inner.connected_peers.store(count, Ordering::Relaxed);
    }

    // ===== Custom Metrics =====

    #[cfg(test)]
    pub fn set_custom_metric(&self, name: String, value: f64) {
        if let Ok(mut metrics) = self.inner.custom_metrics.write() {
            metrics.insert(name, value);
        }
    }

    #[cfg(test)]
    pub fn get_custom_metric(&self, name: &str) -> Option<f64> {
        self.inner.custom_metrics.read().ok()?.get(name).copied()
    }

    // ===== Human-readable Summary =====

    #[cfg(test)]
    pub fn summary(&self) -> String {
        let snapshot = self.snapshot();
        let mut output = String::new();

        output.push_str("=== DAG Consensus Metrics ===\n\n");

        output.push_str("Counters:\n");
        let counter_lines = [
            ("Vertices Created", snapshot.vertices_created),
            ("Vertices Broadcast", snapshot.vertices_broadcast),
            (
                "Vertices Received",
                Self::counter(&self.inner.vertices_received),
            ),
            ("Checkpoints Created", snapshot.checkpoints_created),
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
            ("Active Vertices", snapshot.active_vertices),
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

        output.push_str(&format!("\nUptime: {:.2}s\n", snapshot.uptime_seconds));

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
    fn test_snapshot() {
        let metrics = DagMetrics::new();
        metrics.inc_vertices_created();
        metrics.inc_checkpoints_created();
        metrics.set_active_vertices(7);

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.vertices_created, 1);
        assert_eq!(snapshot.checkpoints_created, 1);
        assert_eq!(snapshot.active_vertices, 7);
        assert!(snapshot.uptime_seconds >= 0.0);
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

        let export = metrics.export_prometheus().unwrap();

        assert!(export.contains("dag_vertices_created_total"));
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
