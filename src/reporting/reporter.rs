use crate::errors::ErrorKind;
use crate::importing::events::MessageTiming;
use crate::reporting::metrics::Metrics;
use futures_util::core_reexport::sync::atomic::Ordering;
use prometheus::core::{Atomic, AtomicF64, AtomicU64};
use std::collections::VecDeque;
use std::sync::RwLock;
use std::time::Instant;

pub(crate) struct Reporter {
    pub(crate) metrics: Metrics,
    pub(crate) last_messages: RwLock<VecDeque<Instant>>,
    pub(crate) last_timing: MessageTiming,

    pub(crate) msg_count: AtomicU64,
    pub(crate) error_count: AtomicU64,
    pub(crate) msg_per_second: AtomicF64,

    pub(crate) acc_poll_time: AtomicF64,
    pub(crate) acc_parse_time: AtomicF64,
    pub(crate) acc_delta_time: AtomicF64,
    pub(crate) acc_fetch_time: AtomicF64,
    pub(crate) acc_insert_time: AtomicF64,
}

impl Reporter {
    pub(crate) fn update_processing_rate(&self) {
        self.msg_count.inc_by(1);
        match self.last_messages.write() {
            Ok(mut last_messages) => {
                let last_message = last_messages.pop_front().unwrap();
                let current_time = Instant::now();
                last_messages.push_back(current_time);
                let next = 1000f64 / current_time.duration_since(last_message).as_secs_f64();
                self.msg_per_second.swap(next, Ordering::Relaxed);
            }
            Err(e) => {
                error!(target: "apex", "Error reporting: {}", e);
                panic!("Couldn't acquire lock while updating processing rate");
            }
        }
    }

    pub(crate) fn add_timing(&self, timing: MessageTiming) {
        self.metrics.message_count_metric.inc();

        debug!(target: "apex", "timing test");
        debug!(target: "apex", "timing poll s: {}", timing.poll_time.as_secs_f64());
        debug!(target: "apex", "timing parse s: {}", timing.parse_time.as_secs_f64());
        debug!(target: "apex", "timing delta s: {}", timing.delta_total().as_secs_f64());
        debug!(target: "apex", "timing fetch s: {}", timing.fetch_time.as_secs_f64());
        debug!(target: "apex", "timing insert s: {}", timing.insert_time.as_secs_f64());

        self.metrics
            .poll_time_metric
            .observe(timing.poll_time.as_secs_f64());
        self.metrics
            .parse_time_metric
            .observe(timing.parse_time.as_secs_f64());
        self.metrics
            .delta_time_metric
            .observe(timing.delta_total().as_secs_f64());
        self.metrics
            .fetch_time_metric
            .observe(timing.fetch_time.as_secs_f64());
        self.metrics
            .insert_time_metric
            .observe(timing.insert_time.as_secs_f64());

        self.acc_poll_time
            .inc_by(timing.poll_time.as_millis() as f64);
        self.acc_parse_time
            .inc_by(timing.parse_time.as_millis() as f64);
        self.acc_delta_time
            .inc_by(timing.delta_total().as_millis() as f64);
        self.acc_fetch_time
            .inc_by(timing.fetch_time.as_millis() as f64);
        self.acc_insert_time
            .inc_by(timing.insert_time.as_millis() as f64);
        // self.last_timing = timing;
    }

    pub(crate) fn add_error(&self, e: ErrorKind) {
        error!(target: "apex", "{}", e);
        self.metrics.error_count_metric.inc();
        self.error_count.inc_by(1);
    }
}

impl Default for Reporter {
    fn default() -> Reporter {
        let mut queue = VecDeque::new();
        queue.reserve_exact(1_000);
        for _ in 0..1_000 {
            queue.push_back(Instant::now())
        }

        Reporter {
            metrics: Default::default(),
            msg_count: AtomicU64::new(0),
            error_count: AtomicU64::new(0),
            msg_per_second: AtomicF64::new(0.0),
            acc_poll_time: AtomicF64::new(0.0),
            acc_parse_time: AtomicF64::new(0.0),
            acc_delta_time: AtomicF64::new(0.0),
            acc_fetch_time: AtomicF64::new(0.0),
            acc_insert_time: AtomicF64::new(0.0),
            last_timing: MessageTiming::new(),
            last_messages: RwLock::new(queue),
        }
    }
}
