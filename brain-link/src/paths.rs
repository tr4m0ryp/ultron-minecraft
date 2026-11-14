use std::path::PathBuf;

/// Environment variable naming the executor socket.
pub const SOCKET_ENV: &str = "ULTRON_BRAIN_SOCK";

/// `$ULTRON_BRAIN_SOCK`, else `$XDG_RUNTIME_DIR/ultron/brain.sock`, else
/// `/tmp/ultron-$USER/brain.sock`.
pub fn default_socket_path() -> PathBuf {
    if let Some(p) = std::env::var_os(SOCKET_ENV).filter(|p| !p.is_empty()) {
        return PathBuf::from(p);
    }
    if let Some(d) = std::env::var_os("XDG_RUNTIME_DIR").filter(|p| !p.is_empty()) {
        return PathBuf::from(d).join("ultron").join("brain.sock");
    }
    let user = std::env::var("USER").unwrap_or_else(|_| "user".into());
    PathBuf::from(format!("/tmp/ultron-{user}")).join("brain.sock")
}
