# Cabinet demo

Kitchen-sink sample. Full walkthrough: [docs → Examples](https://s00d.github.io/sova/examples).

```bash
cp .env.example .env
cargo sovax db migrate -p cabinet && cargo sovax db seed -p cabinet
cargo run -p cabinet
```

Seed user: `demo@sova.local` / `demo1234`. Config: `sova.toml`.
