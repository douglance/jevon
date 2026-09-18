//! The `typesafe` executable.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    jevon::build_cli().serve().await
}
