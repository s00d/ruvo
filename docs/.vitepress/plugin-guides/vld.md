**When:** validate request bodies / forms with typed schemas.

**Does:**
- `vld::schema!` + `req.validate()`
- Optional flash, i18n, OpenAPI hooks
- Form + JSON

### Example

```rust
vld::schema! {
    pub struct CreateUser {
        pub email: String => vld::string().email(),
    }
}
let body: CreateUser = req.validate().await?;
```
