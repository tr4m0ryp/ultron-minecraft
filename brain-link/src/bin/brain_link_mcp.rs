//! MCP tool server for Ultron's thought layer in the game. Connects to the executor socket named by
//! `ULTRON_BRAIN_SOCK` (default under `$XDG_RUNTIME_DIR`).

fn main() {
    let stdin = std::io::stdin().lock();
    let stdout = std::io::stdout().lock();
    if let Err(e) = brain_link::mcp::serve(stdin, stdout, brain_link::default_socket_path()) {
        eprintln!("brain-link-mcp: {e}");
        std::process::exit(1);
    }
}
