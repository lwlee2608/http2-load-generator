use std::sync::atomic::AtomicU32;
use std::sync::atomic::AtomicU64;

pub struct ApiStats {
    success_counter: AtomicU32,
    error_counter: AtomicU32,
    total_rtt: AtomicU64,
    total_retry: AtomicU32,
}

unsafe impl Sync for ApiStats {}
unsafe impl Send for ApiStats {}

impl ApiStats {
    pub fn new() -> ApiStats {
        ApiStats {
            success_counter: AtomicU32::new(0),
            error_counter: AtomicU32::new(0),
            total_rtt: AtomicU64::new(0),
            total_retry: AtomicU32::new(0),
        }
    }

    pub fn inc_success(&self) {
        self.success_counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn get_success(&self) -> u32 {
        self.success_counter
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    // TODO
    // pub fn inc_error(&self) {
    //     self.error_counter
    //         .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    // }

    pub fn get_error(&self) -> u32 {
        self.error_counter
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn inc_rtt(&self, rtt: u64) {
        self.total_rtt
            .fetch_add(rtt, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn get_rtt(&self) -> u64 {
        self.total_rtt.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn inc_retry(&self, retry: u32) {
        self.total_retry
            .fetch_add(retry, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn get_retry(&self) -> u32 {
        self.total_retry.load(std::sync::atomic::Ordering::Relaxed)
    }
}
