//! The `jev` executable.
//!
//! Deliberately empty beyond the entry point: the command graph lives in the
//! library so tests can drive it through `serve_to`, which is the same path
//! this takes. A test that went around it could pass while the binary broke.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    jevon::build_cli().serve().await
}
