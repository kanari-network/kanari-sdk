// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Rate limiting for API endpoints
//!
//! This module provides IP-based rate limiting to prevent brute force attacks
//! and API abuse using a simple token bucket algorithm.

use std::{
    collections::HashMap,
    net::IpAddr,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;
use tracing::warn;

/// Rate limit configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum number of requests per interval
    pub max_requests: u32,
    /// Interval duration in seconds
    pub interval_secs: u64,
}

impl RateLimitConfig {
    /// Default rate limit for login/register endpoints (10 requests per minute)
    pub fn strict() -> Self {
        Self {
            max_requests: 10,
            interval_secs: 60,
        }
    }

    /// Default rate limit for general endpoints (60 requests per minute)
    pub fn moderate() -> Self {
        Self {
            max_requests: 60,
            interval_secs: 60,
        }
    }

    /// Relaxed rate limit for read-only endpoints (120 requests per minute)
    pub fn relaxed() -> Self {
        Self {
            max_requests: 120,
            interval_secs: 60,
        }
    }
}

/// Token bucket for rate limiting a single IP
#[derive(Debug)]
struct TokenBucket {
    max_tokens: u32,
    tokens: u32,
    last_refill: Instant,
    refill_interval: Duration,
}

impl TokenBucket {
    fn new(max_requests: u32, interval_secs: u64) -> Self {
        Self {
            max_tokens: max_requests,
            tokens: max_requests,
            last_refill: Instant::now(),
            refill_interval: Duration::from_secs(interval_secs),
        }
    }

    fn allow_request(&mut self) -> bool {
        let now = Instant::now();

        // Refill tokens based on elapsed time
        let elapsed = now.duration_since(self.last_refill);
        if elapsed >= self.refill_interval {
            self.tokens = self.max_tokens;
            self.last_refill = now;
        }

        // Try to consume a token
        if self.tokens > 0 {
            self.tokens -= 1;
            true
        } else {
            false
        }
    }
}

/// Rate limiter state shared across all handlers
#[derive(Clone)]
pub struct RateLimiter {
    /// Configuration
    config: RateLimitConfig,

    /// Map of IP addresses to their token buckets
    buckets: Arc<Mutex<HashMap<IpAddr, TokenBucket>>>,
}

#[derive(Debug, Clone, Copy)]
pub struct RateLimitError {
    pub retry_after_secs: u64,
}

impl RateLimiter {
    /// Create a new rate limiter with the given configuration
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            buckets: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Check if a request from the given IP is allowed
    pub async fn check_rate_limit(&self, ip: IpAddr) -> Result<(), RateLimitError> {
        let mut buckets = self.buckets.lock().await;

        // Get or create token bucket for this IP
        let bucket = buckets.entry(ip).or_insert_with(|| {
            TokenBucket::new(self.config.max_requests, self.config.interval_secs)
        });

        // Check if request is allowed
        if bucket.allow_request() {
            Ok(())
        } else {
            warn!("Rate limit exceeded for IP: {}", ip);
            Err(RateLimitError {
                retry_after_secs: self.config.interval_secs,
            })
        }
    }

    /// Get current rate limit configuration
    pub fn config(&self) -> &RateLimitConfig {
        &self.config
    }
}
