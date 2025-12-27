use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

const SOCKET_PATH: &str = "/tmp/moonstone.sock";
const HEARTBEAT_INTERVAL: Duration = Duration::from_millis(500);
const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Error, Debug)]
pub enum IpcError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Connection timeout")]
    Timeout,
    #[error("Heartbeat missed")]
    HeartbeatMissed,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Message {
    Heartbeat,
    Shutdown,
    Status,
    EmergencyDisable,
}

impl Message {
    fn to_byte(self) -> u8 {
        match self {
            Message::Heartbeat => 0x01,
            Message::Shutdown => 0x02,
            Message::Status => 0x03,
            Message::EmergencyDisable => 0x04,
        }
    }

    fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            0x01 => Some(Message::Heartbeat),
            0x02 => Some(Message::Shutdown),
            0x03 => Some(Message::Status),
            0x04 => Some(Message::EmergencyDisable),
            _ => None,
        }
    }
}

pub fn socket_path() -> PathBuf {
    PathBuf::from(SOCKET_PATH)
}

/// Server side - runs in the daemon
pub struct IpcServer {
    listener: UnixListener,
    running: Arc<AtomicBool>,
}

impl IpcServer {
    pub fn new() -> Result<Self, IpcError> {
        // Remove existing socket if present
        let path = socket_path();
        let _ = std::fs::remove_file(&path);

        let listener = UnixListener::bind(&path)?;
        listener.set_nonblocking(true)?;

        info!("IPC server listening on {:?}", path);

        Ok(Self {
            listener,
            running: Arc::new(AtomicBool::new(true)),
        })
    }

    /// Run the server, returning a channel for received messages
    pub fn run(&self) -> mpsc::Receiver<(Message, Option<UnixStream>)> {
        let (tx, rx) = mpsc::channel(32);
        let running = self.running.clone();

        // Clone the listener's fd for the spawned task
        let listener = self.listener.try_clone().expect("Failed to clone listener");

        std::thread::spawn(move || {
            while running.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let mut buf = [0u8; 1];
                        if stream.read_exact(&mut buf).is_ok() {
                            if let Some(msg) = Message::from_byte(buf[0]) {
                                debug!("Received message: {:?}", msg);
                                let _ = tx.blocking_send((msg, Some(stream)));
                            }
                        }
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(50));
                    }
                    Err(e) => {
                        error!("Accept error: {}", e);
                    }
                }
            }
        });

        rx
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

impl Drop for IpcServer {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(socket_path());
    }
}

/// Client side - used by watchdog and CLI
pub struct IpcClient {
    last_heartbeat: Instant,
}

impl IpcClient {
    pub fn new() -> Self {
        Self {
            last_heartbeat: Instant::now(),
        }
    }

    /// Send a message to the daemon
    pub fn send(&self, msg: Message) -> Result<(), IpcError> {
        let mut stream = UnixStream::connect(socket_path())?;
        stream.set_write_timeout(Some(Duration::from_secs(1)))?;
        stream.write_all(&[msg.to_byte()])?;
        Ok(())
    }

    /// Send heartbeat and check if daemon is alive
    pub fn heartbeat(&mut self) -> Result<(), IpcError> {
        self.send(Message::Heartbeat)?;
        self.last_heartbeat = Instant::now();
        Ok(())
    }

    /// Check if we've missed too many heartbeats
    pub fn is_daemon_alive(&self) -> bool {
        self.last_heartbeat.elapsed() < HEARTBEAT_TIMEOUT
    }

    /// Run heartbeat loop, returns error if daemon stops responding
    pub async fn run_heartbeat_loop(&mut self) -> IpcError {
        let mut interval = tokio::time::interval(HEARTBEAT_INTERVAL);
        let mut consecutive_failures = 0;

        loop {
            interval.tick().await;

            match self.heartbeat() {
                Ok(()) => {
                    consecutive_failures = 0;
                    debug!("Heartbeat OK");
                }
                Err(e) => {
                    consecutive_failures += 1;
                    warn!(
                        "Heartbeat failed ({}/4): {}",
                        consecutive_failures, e
                    );

                    if consecutive_failures >= 4 {
                        error!("Daemon not responding - triggering tamper response");
                        return IpcError::HeartbeatMissed;
                    }
                }
            }
        }
    }
}

impl Default for IpcClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if daemon is running
pub fn is_daemon_running() -> bool {
    IpcClient::new().send(Message::Status).is_ok()
}
