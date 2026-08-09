[![crates.io](https://img.shields.io/crates/v/sova?style=for-the-badge)](https://crates.io/crates/sova)
[![docs.rs](https://img.shields.io/docsrs/sova?style=for-the-badge)](https://docs.rs/sova)
[![License](https://img.shields.io/crates/l/sova?style=for-the-badge)](https://github.com/s00d/sova/blob/master/LICENSE)
[![Donate](https://img.shields.io/badge/Donate-Donationalerts-ff4081?style=for-the-badge)](https://www.donationalerts.com/r/s00d88)

# Cabinet demo

Kitchen-sink sample. Full walkthrough: [docs → Examples](https://s00d.github.io/sova/examples).

```bash
cp .env.example .env
cargo sovax db migrate -p cabinet && cargo sovax db seed -p cabinet
cargo run -p cabinet
```

Seed user: `demo@sova.local` / `demo1234`. Config: `sova.toml`.
