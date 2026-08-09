# Deploy with Docker Compose

**Start here:** [`docker-compose.yml`](./docker-compose.yml).

```bash
# from this directory
cp .env.example .env   # optional
docker compose pull    # ghcr.io/s00d/sova-hello:latest
docker compose up -d
curl -sS http://127.0.0.1:3000/
docker compose down

# or build locally instead of pulling
docker compose up --build -d
```

From the repo root:

```bash
docker compose -f deploy/docker-compose.yml up --build -d
```

| File | |
|------|--|
| [docker-compose.yml](./docker-compose.yml) | compose entrypoint |
| [Dockerfile](./Dockerfile) | multi-stage image build |
| [sova.production.toml](./sova.production.toml) | mounted at `/app/sova.toml` |
| [.env.example](./.env.example) | env template |

Docs: [Production / Docker](https://s00d.github.io/sova/guide/production.html).
