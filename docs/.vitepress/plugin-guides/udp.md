**When:** datagram listeners as background services (not HTTP).

**Does:**
- `UdpService` binds + handler per packet
- Built-in `echo` helper
- Shutdown-aware via `app.service(...)`

### Example

```rust
use sova::{App, UdpService};
use std::net::SocketAddr;

let mut app = App::new();
app.service(UdpService::echo("127.0.0.1:9999".parse()?));
app.listen(3011).await?;
```
