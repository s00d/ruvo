Not part of web/api presets — register as a background service:

```rust
use sova::{App, Result, UdpService};
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = App::new();
    let addr: SocketAddr = "127.0.0.1:9999".parse().unwrap();
    app.service(UdpService::echo(addr));
    app.get("/", |_| async { sova::Response::text("udp echo on :9999") });
    app.listen(3011).await
}
```

```bash
cargo run -p udp_echo
```
