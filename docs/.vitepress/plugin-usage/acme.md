```rust
use sova::{Acme, App};

let acme = Acme::lets_encrypt(["example.com"])
    .email("ops@example.com")
    .dir("./data/acme");
let tls = acme.tls()?;
app.install(acme.with_tls(tls.clone()));
// app.bind("0.0.0.0:443").tls(tls)?.run().await?;
```
