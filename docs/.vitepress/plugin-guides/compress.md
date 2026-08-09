**When:** shrink HTML/JSON/static responses (gzip / deflate / brotli).

**Does:**
- Negotiates encoding from `Accept-Encoding`
- Buffered body compression (not chunk-streamed)
- Threshold + level + custom filter

### Example

```rust
app.install(Compress::new().threshold(1024).level(6));
```
