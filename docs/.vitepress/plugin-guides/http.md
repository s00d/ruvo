**When:** call upstream HTTP APIs from handlers (with SSRF guards).

**Does:**
- `OutboundHttp` plugin + `req.http()`
- Named clients / configs, request-bound deadline, tracing
- Fake transport for tests

### Example

```rust
app.install(OutboundHttp::new());
let upstream = req.http().get("https://example.com/api").send().await?;
```
