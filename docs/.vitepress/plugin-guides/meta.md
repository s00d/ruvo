**When:** document title, description, OG/Twitter, JSON-LD, head inject.

**Does:**
- `Meta::page().title(…).description(…)`
- Route/router `.with(Meta::…)`
- Sitemap / robots helpers in crate

### Example

```rust
app.get("/", home).with(Meta::page().title("Home").description("Welcome"));
```
