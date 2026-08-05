use duet_types::VPath;
use std::time::Instant;

/// Dual-regime Exponentially Weighted Moving Average (EMA) progress & ETA calculator.
#[derive(Debug)]
pub struct ProgressTracker {
    pub total_files: u64,
    pub total_bytes: u64,
    pub files_processed: u64,
    pub bytes_transferred: u64,
    pub current_file: Option<VPath>,

    start_time: Instant,
    last_sample_time: Instant,
    last_bytes: u64,
    last_files: u64,

    byte_rate_ema: f64,
    file_rate_ema: f64,
    alpha: f64, // Smoothing factor, default 0.2
}

impl ProgressTracker {
    pub fn new(total_files: u64, total_bytes: u64) -> Self {
        let now = Instant::now();
        Self {
            total_files,
            total_bytes,
            files_processed: 0,
            bytes_transferred: 0,
            current_file: None,
            start_time: now,
            last_sample_time: now,
            last_bytes: 0,
            last_files: 0,
            byte_rate_ema: 0.0,
            file_rate_ema: 0.0,
            alpha: 0.2,
        }
    }

    /// Update progress counters and recalculate dual-regime EMA rates.
    pub fn update(&mut self, bytes_delta: u64, files_delta: u64, current_file: Option<VPath>) {
        self.bytes_transferred += bytes_delta;
        self.files_processed += files_delta;
        self.current_file = current_file;

        let now = Instant::now();
        let elapsed = now.duration_since(self.last_sample_time).as_secs_f64();

        // Sample every >= 100 ms
        if elapsed >= 0.1 {
            let bytes_since = self.bytes_transferred - self.last_bytes;
            let files_since = self.files_processed - self.last_files;

            let current_byte_rate = bytes_since as f64 / elapsed;
            let current_file_rate = files_since as f64 / elapsed;

            if self.byte_rate_ema == 0.0 {
                self.byte_rate_ema = current_byte_rate;
            } else {
                self.byte_rate_ema =
                    self.alpha * current_byte_rate + (1.0 - self.alpha) * self.byte_rate_ema;
            }

            if self.file_rate_ema == 0.0 {
                self.file_rate_ema = current_file_rate;
            } else {
                self.file_rate_ema =
                    self.alpha * current_file_rate + (1.0 - self.alpha) * self.file_rate_ema;
            }

            self.last_sample_time = now;
            self.last_bytes = self.bytes_transferred;
            self.last_files = self.files_processed;
        }
    }

    /// Calculate estimated remaining time in seconds using dual-regime blend.
    pub fn eta_seconds(&self) -> Option<f64> {
        if self.bytes_transferred >= self.total_bytes && self.files_processed >= self.total_files {
            return Some(0.0);
        }

        let total_elapsed = self.start_time.elapsed().as_secs_f64();
        if total_elapsed < 0.5 {
            return None; // Wait for initial sampling
        }

        let remaining_bytes = self.total_bytes.saturating_sub(self.bytes_transferred) as f64;
        let remaining_files = self.total_files.saturating_sub(self.files_processed) as f64;

        // Determine average file size
        let avg_file_size = if self.total_files > 0 {
            self.total_bytes as f64 / self.total_files as f64
        } else {
            0.0
        };

        // Small-file regime threshold: files < 128 KiB average
        let small_file_threshold = 128.0 * 1024.0;

        if avg_file_size < small_file_threshold && self.file_rate_ema > 0.0 {
            // Small file regime: file overhead dominates
            let eta = remaining_files / self.file_rate_ema;
            Some(eta.max(0.0))
        } else if self.byte_rate_ema > 0.0 {
            // Large file regime: byte throughput dominates
            let eta = remaining_bytes / self.byte_rate_ema;
            Some(eta.max(0.0))
        } else {
            None
        }
    }
}
