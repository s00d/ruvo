**When:** MiniJinja HTML views. Already on `App::web()`.

**Does:**
- Render templates from `views/`
- Optional autoreload in dev
- Shared with mail templates feature

### Example

```rust
app.install(Templates::new("views"));
Ok(req.render("home.html", json!({ "title": "Hi" }))?)
```
