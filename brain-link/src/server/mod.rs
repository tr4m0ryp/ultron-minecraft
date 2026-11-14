//! Unix socket server: one thread accepts, one thread per client reads request lines and
//! writes one reply line each.

mod conn;

use std::fs;
use std::io;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

use crate::bridge::Bridge;

/// Owns the listening socket. Dropping it stops accepting and removes the socket file.
pub struct Server {
    path: PathBuf,
    stop: Arc<AtomicBool>,
    accept: Option<JoinHandle<()>>,
}

impl Server {
    pub fn start(path: &Path, bridge: Arc<Bridge>) -> io::Result<Server> {
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            fs::DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(dir)?;
        }
        if path.exists() {
            if UnixStream::connect(path).is_ok() {
                return Err(io::Error::new(
                    io::ErrorKind::AddrInUse,
                    format!("another executor already serves {}", path.display()),
                ));
            }
            fs::remove_file(path)?;
        }
        let listener = UnixListener::bind(path)?;
        // The socket steers the bot, so only this user may connect.
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let accept = std::thread::Builder::new()
            .name("brain-link-accept".into())
            .spawn(move || {
                for stream in listener.incoming() {
                    if flag.load(Ordering::SeqCst) {
                        break;
                    }
                    let Ok(stream) = stream else { continue };
                    let bridge = bridge.clone();
                    let _ = std::thread::Builder::new()
                        .name("brain-link-conn".into())
                        .spawn(move || conn::serve(stream, &bridge));
                }
            })?;
        Ok(Server {
            path: path.to_path_buf(),
            stop,
            accept: Some(accept),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        // Wakes the blocking accept so the thread sees the flag.
        let _ = UnixStream::connect(&self.path);
        if let Some(h) = self.accept.take() {
            let _ = h.join();
        }
        let _ = fs::remove_file(&self.path);
    }
}
