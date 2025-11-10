use std::fs::{File, create_dir_all};
use std::io::{self, Write};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Log event types that determine which receivers should log the message
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogEvent {
    /// Simulation iteration data (detailed per-iteration info)
    Simulation,
    /// Convergence information (iteration counts, convergence messages)
    Convergence,
    /// Variant-level data (final converged simulation results for a variant)
    Variant,
    /// Scenario-level data (comparisons between variants, scenario summaries)
    Scenario,
    /// Validation results (pass/fail messages, validation checks)
    Validation,
}

/// Trait for log receivers that can receive log messages
pub trait LogReceiver: Send {
    /// Check if this receiver should handle the given log event
    fn should_log(&self, event: LogEvent) -> bool;
    
    /// Write a string to this receiver
    fn write(&mut self, s: &str) -> io::Result<()>;
    
    /// Flush this receiver
    fn flush(&mut self) -> io::Result<()>;
}

/// Console log receiver (writes to stdout)
pub struct ConsoleReceiver {
    enabled_events: Vec<LogEvent>,
}

impl ConsoleReceiver {
    /// Create a new console receiver
    /// Returns a boxed receiver ready to be added to a logger
    pub fn new(enabled_events: Vec<LogEvent>) -> Box<dyn LogReceiver> {
        Box::new(Self { enabled_events })
    }
}

impl LogReceiver for ConsoleReceiver {
    fn should_log(&self, event: LogEvent) -> bool {
        self.enabled_events.contains(&event)
    }
    
    fn write(&mut self, s: &str) -> io::Result<()> {
        print!("{}", s);
        io::stdout().flush()
    }
    
    fn flush(&mut self) -> io::Result<()> {
        io::stdout().flush()
    }
}

/// File log receiver (writes to a file)
pub struct FileReceiver {
    file: File,
    enabled_events: Vec<LogEvent>,
}

impl FileReceiver {
    /// Create a new file receiver that writes to the specified path
    /// The file will be created (truncated if it exists) and parent directories will be created if needed
    /// Panics if file creation fails
    /// Returns a boxed receiver ready to be added to a logger
    pub fn new(path: &Path, enabled_events: Vec<LogEvent>) -> Box<dyn LogReceiver> {
        if let Some(parent) = path.parent() {
            create_dir_all(parent).expect("Failed to create log directory");
        }
        let file = File::create(path).expect("Failed to create log file");
        Box::new(Self { file, enabled_events })
    }
}

impl LogReceiver for FileReceiver {
    fn should_log(&self, event: LogEvent) -> bool {
        self.enabled_events.contains(&event)
    }
    
    fn write(&mut self, s: &str) -> io::Result<()> {
        write!(self.file, "{}", s)?;
        self.file.flush()
    }
    
    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

/// Unique identifier for a receiver
pub type ReceiverId = usize;

/// Global counter for generating unique receiver IDs
static RECEIVER_ID_COUNTER: AtomicUsize = AtomicUsize::new(1);

/// Main logger that manages multiple receivers
pub struct Logger {
    receivers: Vec<(ReceiverId, Box<dyn LogReceiver>)>,
}

impl Logger {
    /// Create a new logger with no receivers
    pub fn new() -> Self {
        Self {
            receivers: Vec::new(),
        }
    }
    
    /// Add a receiver to the logger and return its unique ID
    pub fn add_receiver(&mut self, receiver: Box<dyn LogReceiver>) -> ReceiverId {
        let id = RECEIVER_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        self.receivers.push((id, receiver));
        id
    }
    
    /// Remove a receiver by its ID
    pub fn remove_receiver(&mut self, id: ReceiverId) {
        self.receivers.retain(|(receiver_id, _)| *receiver_id != id);
    }
    
    /// Write a message with a specific log event type
    pub fn log(&mut self, event: LogEvent, message: &str) -> io::Result<()> {
        for (_, receiver) in &mut self.receivers {
            if receiver.should_log(event) {
                receiver.write(message)?;
            }
        }
        Ok(())
    }
    
    /// Write a message with newline
    pub fn logln(&mut self, event: LogEvent, message: &str) -> io::Result<()> {
        self.log(event, &format!("{}\n", message))
    }
    
    /// Flush all receivers
    pub fn flush(&mut self) -> io::Result<()> {
        for (_, receiver) in &mut self.receivers {
            receiver.flush()?;
        }
        Ok(())
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self::new()
    }
}


/// Sanitize a string to be used as a filename
pub fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            ' ' | '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect()
}

/// Macro to log a formatted string (like println! but for logger)
#[macro_export]
macro_rules! logln {
    ($logger:expr, $event:expr, $($arg:tt)*) => {
        {
            let _ = $logger.logln($event, &format!($($arg)*));
        }
    };
}

/// Macro to log a formatted string without newline (like print! but for logger)
#[macro_export]
macro_rules! log {
    ($logger:expr, $event:expr, $($arg:tt)*) => {
        {
            let _ = $logger.log($event, &format!($($arg)*));
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("test name"), "test_name");
        assert_eq!(sanitize_filename("test/name"), "test_name");
        assert_eq!(sanitize_filename("test:name"), "test_name");
    }
}

