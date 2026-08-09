# Deploy / Docker

Multi-stage image and Compose for a production-shaped Sova process.

```bash
# from repo root
docker compose -f deploy/docker-compose.yml up --build -d
curl -sS http://127.0.0.1:3000/
docker compose -f deploy/docker-compose.yml down
```

Or `cd deploy && docker compose up --build`.

Default package: `examples/basic/hello`. Override build args `EXAMPLE_PKG` / `EXAMPLE_BIN`.

`/app/sova.toml` is copied into the image for copy-paste. The framework has no `SOVA_CONFIG` env — apps must call `configure()` / `configure_from_path` themselves. The default `hello` example does not load toml. Uncomment the compose volume if your app reads `/app/sova.toml`.

See [Production / Docker](https://s00d.github.io/sova/guide/production.html).
