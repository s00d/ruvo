**When:** OpenAPI 3.1 + Scalar UI for APIs.

**Does:**
- Document from routes / vld schemas
- UI at mount path

### Example

```rust
app.install(OpenApi::new().mount("/docs"));
```
