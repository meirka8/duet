use log::{LevelFilter, SetLoggerError};
use std::cell::Cell;
use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

thread_local! {
    static UI_THREAD: Cell<bool> = const { Cell::new(false) };
}

/// Sets whether the current thread is the UI thread.
pub fn set_ui_thread(is_ui: bool) {
    UI_THREAD.with(|cell| cell.set(is_ui));
}

/// Asserts that the current thread is not the UI thread.
/// Panics in debug builds if called from the UI thread.
pub fn assert_not_ui_thread() {
    #[cfg(debug_assertions)]
    {
        UI_THREAD.with(|cell| {
            if cell.get() {
                panic!("Blocking operation called on the UI thread!");
            }
        });
    }
}

/// Structured log event captured in circular ring buffer.
#[derive(Debug, Clone)]
pub struct LogEvent {
    pub timestamp_millis: u64,
    pub level: log::Level,
    pub target: String,
    pub message: String,
}

static LOG_BUFFER: OnceLock<Arc<Mutex<VecDeque<LogEvent>>>> = OnceLock::new();
static CRASH_DIR: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();

pub struct RingBufferLogger {
    capacity: usize,
    buffer: Arc<Mutex<VecDeque<LogEvent>>>,
}

impl log::Log for RingBufferLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let event = LogEvent {
            timestamp_millis: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            level: record.level(),
            target: record.target().to_string(),
            message: format!("{}", record.args()),
        };

        if let Ok(mut buf) = self.buffer.lock() {
            if buf.len() >= self.capacity {
                buf.pop_front();
            }
            buf.push_back(event);
        }
    }

    fn flush(&self) {}
}

/// Initialize global logging with circular ring buffer (200 events) and panic hook crash file writer (T-3.3.3).
pub fn init_logging_and_crash_handler(
    crash_dir_override: Option<PathBuf>,
) -> Result<(), SetLoggerError> {
    let buffer = Arc::new(Mutex::new(VecDeque::with_capacity(200)));
    let _ = LOG_BUFFER.set(buffer.clone());

    if let Some(override_dir) = crash_dir_override {
        if let Ok(mut guard) = CRASH_DIR.get_or_init(|| Mutex::new(None)).lock() {
            *guard = Some(override_dir);
        }
    }

    let logger = RingBufferLogger {
        capacity: 200,
        buffer,
    };

    log::set_boxed_logger(Box::new(logger))?;
    log::set_max_level(LevelFilter::Debug);

    setup_panic_hook();

    Ok(())
}

/// Fetch all recent log events stored in the circular ring buffer.
pub fn get_recent_log_events() -> Vec<LogEvent> {
    if let Some(buffer) = LOG_BUFFER.get() {
        if let Ok(buf) = buffer.lock() {
            return buf.iter().cloned().collect();
        }
    }
    Vec::new()
}

/// Internal helper writing crash report file to `~/.local/state/duet/crashes/` upon panic.
pub fn write_crash_report(panic_info: &std::panic::PanicHookInfo<'_>) -> Option<PathBuf> {
    let payload = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
        s.clone()
    } else {
        "Unknown panic payload".to_string()
    };

    let location = panic_info
        .location()
        .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
        .unwrap_or_else(|| "unknown location".to_string());

    let backtrace = std::backtrace::Backtrace::capture();

    let logs = get_recent_log_events();

    let mut report = String::new();
    report.push_str("====================================================\n");
    report.push_str("              DUET FILE MANAGER CRASH REPORT        \n");
    report.push_str("====================================================\n\n");
    report.push_str(&format!("Panic Payload: {}\n", payload));
    report.push_str(&format!("Location: {}\n\n", location));
    report.push_str("----- BACKTRACE -----\n");
    report.push_str(&format!("{}\n\n", backtrace));
    report.push_str("----- RECENT TRACE EVENTS (LAST 200) -----\n");
    for ev in &logs {
        report.push_str(&format!(
            "[{}] [{}] [{}] {}\n",
            ev.timestamp_millis, ev.level, ev.target, ev.message
        ));
    }
    report.push_str("====================================================\n");

    let crash_dir = if let Some(cell) = CRASH_DIR.get() {
        if let Ok(guard) = cell.lock() {
            guard.clone()
        } else {
            None
        }
    } else {
        None
    }
    .unwrap_or_else(|| {
        if let Some(home) = std::env::var_os("HOME") {
            PathBuf::from(home)
                .join(".local")
                .join("state")
                .join("duet")
                .join("crashes")
        } else {
            PathBuf::from("/tmp/duet/crashes")
        }
    });

    let _ = fs::create_dir_all(&crash_dir);
    let filename = format!(
        "crash_{}.log",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );
    let crash_file = crash_dir.join(filename);

    if fs::write(&crash_file, report).is_ok() {
        Some(crash_file)
    } else {
        None
    }
}

fn setup_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = write_crash_report(info);
        default_hook(info);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    #[should_panic(expected = "Blocking operation called on the UI thread!")]
    fn test_ui_thread_guard_panics_on_ui_thread() {
        set_ui_thread(true);
        assert_not_ui_thread();
    }

    #[test]
    fn test_ui_thread_guard_does_not_panic_on_non_ui_thread() {
        set_ui_thread(false);
        assert_not_ui_thread();
    }

    #[test]
    fn test_circular_log_buffer_and_crash_writer() {
        let temp = tempdir().unwrap();
        let crash_dir = temp.path().join("crashes");

        let _ = init_logging_and_crash_handler(Some(crash_dir.clone()));

        log::info!("Test info message 1");
        log::warn!("Test warning message 2");

        let logs = get_recent_log_events();
        assert!(logs.iter().any(|l| l.message.contains("Test info message 1")));
        assert!(logs.iter().any(|l| l.message.contains("Test warning message 2")));
    }
}
