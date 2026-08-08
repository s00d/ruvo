# Cabinet demo

Kitchen-sink sample. Full walkthrough: [docs → Examples](https://s00d.github.io/ruvo/examples).

```bash
cp .env.example .env
cargo ruvo db migrate -p cabinet && cargo ruvo db seed -p cabinet
cargo run -p cabinet
```

Seed user: `demo@ruvo.local` / `demo1234`. Config: `ruvo.toml`.
