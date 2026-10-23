use reqwest::{header::RETRY_AFTER, Response, StatusCode};
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, Semaphore, SemaphorePermit};
use tokio::time::{sleep, Instant};

tokio::task_local! {
    static BACKGROUND: bool;
}

pub async fn in_background<F: Future>(work: F) -> F::Output {
    BACKGROUND.scope(true, work).await
}

fn is_background() -> bool {
    BACKGROUND.try_with(|flag| *flag).unwrap_or(false)
}

#[derive(Debug, Clone)]
pub struct ProviderQueueConfig {
    pub max_concurrent: usize,
    pub min_interval: Duration,
    pub max_retries: usize,
    pub default_retry_after: Duration,
    pub max_cooldown: Duration,
}

impl Default for ProviderQueueConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 3,
            min_interval: Duration::from_millis(100),
            max_retries: 3,
            default_retry_after: Duration::from_millis(1500),
            max_cooldown: Duration::from_secs(10),
        }
    }
}

impl ProviderQueueConfig {
    pub fn with_max_concurrent(mut self, max: usize) -> Self {
        self.max_concurrent = max.max(1);
        self
    }

    pub fn with_min_interval(mut self, interval: Duration) -> Self {
        self.min_interval = interval;
        self
    }

    pub fn with_max_retries(mut self, retries: usize) -> Self {
        self.max_retries = retries;
        self
    }
}

#[derive(Debug)]
struct QueueGate {
    tokens: f64,
    last_refill_at: Option<Instant>,
    cooldown_until: Option<Instant>,
}

impl QueueGate {
    fn refill(&mut self, now: Instant, rate_per_second: f64, capacity: f64) {
        let elapsed = match self.last_refill_at {
            Some(previous) => now.saturating_duration_since(previous).as_secs_f64(),
            None => capacity / rate_per_second.max(f64::MIN_POSITIVE),
        };
        self.last_refill_at = Some(now);
        self.tokens = (self.tokens + elapsed * rate_per_second).min(capacity);
    }
}

pub struct ProviderRequestQueue {
    provider_id: String,
    config: ProviderQueueConfig,
    semaphore: Arc<Semaphore>,
    background: Arc<Semaphore>,
    gate: Arc<Mutex<QueueGate>>,
    rate_per_second: f64,
    burst_capacity: f64,
}

impl ProviderRequestQueue {
    pub fn new(provider_id: impl Into<String>, config: ProviderQueueConfig) -> Self {
        let max_concurrent = config.max_concurrent.max(1);
        let interval = config.min_interval.as_secs_f64();
        let rate_per_second = if interval > 0.0 {
            1.0 / interval
        } else {
            f64::INFINITY
        };
        Self {
            provider_id: provider_id.into(),
            config,
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            background: Arc::new(Semaphore::new(max_concurrent.saturating_sub(1).max(1))),
            gate: Arc::new(Mutex::new(QueueGate {
                tokens: max_concurrent as f64,
                last_refill_at: None,
                cooldown_until: None,
            })),
            rate_per_second,
            burst_capacity: max_concurrent as f64,
        }
    }

    pub fn provider_id(&self) -> &str {
        &self.provider_id
    }

    pub fn config(&self) -> &ProviderQueueConfig {
        &self.config
    }

    pub async fn send_request<F>(&self, make_builder: F) -> Result<Response, String>
    where
        F: Fn() -> reqwest::RequestBuilder,
    {
        self.send(move || {
            let req = make_builder();
            async move { req.send().await }
        })
        .await
    }

    pub async fn send<F, Fut>(&self, make_request: F) -> Result<Response, String>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<Response, reqwest::Error>>,
    {
        let _slot = self.acquire_slot().await?;

        let mut attempt = 0;
        loop {
            self.wait_for_slot().await;

            let resp = match make_request().await {
                Ok(r) => r,
                Err(err) => {
                    if (err.is_connect() || err.is_timeout()) && attempt < self.config.max_retries {
                        attempt += 1;
                        let delay = self
                            .config
                            .default_retry_after
                            .min(self.config.max_cooldown);
                        self.set_cooldown(delay).await;
                        tracing::warn!(
                            provider = %self.provider_id,
                            attempt,
                            error = %crate::net::error_chain(&err),
                            "Transient network error; retrying after cooldown"
                        );
                        sleep(delay).await;
                        continue;
                    }
                    return Err(format!(
                        "Provider '{}' request error: {}",
                        self.provider_id,
                        crate::net::error_chain(&err)
                    ));
                }
            };

            let status = resp.status();

            if status == StatusCode::TOO_MANY_REQUESTS && attempt < self.config.max_retries {
                attempt += 1;
                let delay = self.calculate_cooldown(&resp, attempt);
                self.set_cooldown(delay).await;
                tracing::warn!(
                    provider = %self.provider_id,
                    attempt,
                    delay_ms = delay.as_millis(),
                    "Provider received HTTP 429 Too Many Requests; applying queue cooldown"
                );
                sleep(delay).await;
                continue;
            }

            if (status == StatusCode::BAD_GATEWAY
                || status == StatusCode::SERVICE_UNAVAILABLE
                || status == StatusCode::GATEWAY_TIMEOUT)
                && attempt < self.config.max_retries
            {
                attempt += 1;
                let delay = self.calculate_cooldown(&resp, attempt);
                self.set_cooldown(delay).await;
                tracing::warn!(
                    provider = %self.provider_id,
                    status = %status,
                    attempt,
                    delay_ms = delay.as_millis(),
                    "Transient server error; applying queue cooldown and retrying"
                );
                sleep(delay).await;
                continue;
            }

            return Ok(resp);
        }
    }

    async fn acquire_slot(
        &self,
    ) -> Result<(Option<SemaphorePermit<'_>>, SemaphorePermit<'_>), String> {
        let closed = |e| format!("Queue for '{}' closed: {e}", self.provider_id);
        let background = if is_background() {
            Some(self.background.acquire().await.map_err(closed)?)
        } else {
            None
        };
        let permit = self.semaphore.acquire().await.map_err(closed)?;
        Ok((background, permit))
    }

    pub async fn wait_for_slot(&self) {
        loop {
            let sleep_duration = {
                let mut gate = self.gate.lock().await;
                let now = Instant::now();

                if let Some(cooldown) = gate.cooldown_until {
                    if cooldown > now {
                        Some(cooldown - now)
                    } else {
                        gate.cooldown_until = None;
                        self.take_token(&mut gate, now)
                    }
                } else {
                    self.take_token(&mut gate, now)
                }
            };

            if let Some(dur) = sleep_duration {
                sleep(dur).await;
            } else {
                break;
            }
        }
    }

    fn take_token(&self, gate: &mut QueueGate, now: Instant) -> Option<Duration> {
        if !self.rate_per_second.is_finite() {
            return None;
        }

        gate.refill(now, self.rate_per_second, self.burst_capacity);
        if gate.tokens >= 1.0 {
            gate.tokens -= 1.0;
            return None;
        }

        let deficit = 1.0 - gate.tokens;
        Some(Duration::from_secs_f64(deficit / self.rate_per_second))
    }

    fn calculate_cooldown(&self, resp: &Response, attempt: usize) -> Duration {
        if let Some(header) = resp.headers().get(RETRY_AFTER) {
            if let Ok(val) = header.to_str() {
                if let Ok(secs) = val.parse::<u64>() {
                    return Duration::from_secs(secs).min(self.config.max_cooldown);
                }
            }
        }
        let progressive = self.config.default_retry_after * attempt as u32;
        progressive.min(self.config.max_cooldown)
    }

    pub async fn set_cooldown(&self, delay: Duration) {
        let mut gate = self.gate.lock().await;
        let target = Instant::now() + delay;
        match gate.cooldown_until {
            Some(existing) if existing >= target => {}
            _ => gate.cooldown_until = Some(target),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn background_requests_leave_a_slot_for_the_foreground() {
        let queue = Arc::new(ProviderRequestQueue::new(
            "test_prov",
            ProviderQueueConfig::default().with_max_concurrent(3),
        ));
        let prefetches: Vec<_> = (0..6)
            .map(|_| {
                let q = queue.clone();
                tokio::spawn(in_background(async move {
                    let _slot = q.acquire_slot().await.unwrap();
                    sleep(Duration::from_millis(400)).await;
                }))
            })
            .collect();
        sleep(Duration::from_millis(50)).await;

        let started = Instant::now();
        drop(queue.acquire_slot().await.unwrap());
        assert!(
            started.elapsed() < Duration::from_millis(100),
            "foreground waited {:?} behind prefetch",
            started.elapsed()
        );
        for task in prefetches {
            task.await.unwrap();
        }
    }

    #[tokio::test]
    async fn concurrent_requests_never_exceed_max_concurrent() {
        let queue = Arc::new(ProviderRequestQueue::new(
            "test_prov",
            ProviderQueueConfig::default()
                .with_max_concurrent(2)
                .with_min_interval(Duration::from_millis(5)),
        ));
        let active = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));

        let tasks: Vec<_> = (0..6)
            .map(|_| {
                let (q, active, peak) = (queue.clone(), active.clone(), peak.clone());
                tokio::spawn(async move {
                    let _slot = q.acquire_slot().await.unwrap();
                    q.wait_for_slot().await;
                    peak.fetch_max(active.fetch_add(1, Ordering::SeqCst) + 1, Ordering::SeqCst);
                    sleep(Duration::from_millis(30)).await;
                    active.fetch_sub(1, Ordering::SeqCst);
                })
            })
            .collect();
        for task in tasks {
            task.await.unwrap();
        }

        assert!(peak.load(Ordering::SeqCst) <= 2);
    }

    #[tokio::test]
    async fn queue_allows_a_burst_then_throttles_to_the_configured_rate() {
        let queue = ProviderRequestQueue::new(
            "burst",
            ProviderQueueConfig {
                max_concurrent: 3,
                min_interval: Duration::from_millis(100),
                ..ProviderQueueConfig::default()
            },
        );

        let start = Instant::now();
        for _ in 0..3 {
            queue.wait_for_slot().await;
        }
        assert!(
            start.elapsed() < Duration::from_millis(50),
            "burst throttled: {:?}",
            start.elapsed()
        );

        let throttled = Instant::now();
        queue.wait_for_slot().await;
        assert!(
            throttled.elapsed() >= Duration::from_millis(80),
            "rate not enforced: {:?}",
            throttled.elapsed()
        );
    }

    #[tokio::test]
    async fn cooldown_blocks_only_its_own_provider() {
        let cooling = ProviderRequestQueue::new("coomer", ProviderQueueConfig::default());
        let other = ProviderRequestQueue::new("pawchive", ProviderQueueConfig::default());
        cooling.set_cooldown(Duration::from_millis(120)).await;

        let start = Instant::now();
        other.wait_for_slot().await;
        assert!(
            start.elapsed() < Duration::from_millis(20),
            "other provider waited {:?}",
            start.elapsed()
        );

        cooling.wait_for_slot().await;
        assert!(
            start.elapsed() >= Duration::from_millis(100),
            "cooldown not honoured: {:?}",
            start.elapsed()
        );
    }
}
