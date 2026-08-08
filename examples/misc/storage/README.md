# storage_demo

Thin demo for `Storage::from_env()` — memory by default, S3/R2/MinIO with `storage-s3`.

## Memory (no deps)

```bash
RUVO_STORAGE=memory cargo run -p storage_demo
curl -s localhost:3030/
curl -s -X POST localhost:3030/put -H 'content-type: application/json' \
  -d '{"key":"hello.txt","data":"hi"}'
curl -s 'localhost:3030/list?prefix='
curl -s 'localhost:3030/temporary-url?key=hello.txt&expires=60'   # s3 only
```

## MinIO one-liner

```bash
docker run --rm -p 9000:9000 -p 9001:9001 \
  -e MINIO_ROOT_USER=minioadmin \
  -e MINIO_ROOT_PASSWORD=minioadmin \
  minio/minio server /data --console-address ":9001"
```

Create bucket `ruvo` in the console (`http://127.0.0.1:9001`) or with `mc`.

```bash
export RUVO_STORAGE=s3
export RUVO_STORAGE_BUCKET=ruvo
export RUVO_STORAGE_ENDPOINT=http://127.0.0.1:9000
# region defaults to `auto` when ENDPOINT is set; path-style on by default
export AWS_ACCESS_KEY_ID=minioadmin
export AWS_SECRET_ACCESS_KEY=minioadmin

cargo run -p storage_demo
```

Optional: `RUVO_STORAGE_ROOT`, `RUVO_STORAGE_FORCE_PATH_STYLE=0|1`, `AWS_SESSION_TOKEN`,
`RUVO_STORAGE_PUBLIC_URL`.

Smoke test (crate):

```bash
cargo test -p ruvo-storage --features s3 --test minio_smoke -- --ignored --nocapture
```
