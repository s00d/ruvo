[![crates.io](https://img.shields.io/crates/v/sova-rate-limit?style=for-the-badge)](https://crates.io/crates/sova-rate-limit)
[![downloads](https://img.shields.io/crates/d/sova-rate-limit?style=for-the-badge)](https://crates.io/crates/sova-rate-limit)
[![docs.rs](https://img.shields.io/docsrs/sova-rate-limit?style=for-the-badge)](https://docs.rs/sova-rate-limit)
[![License](https://img.shields.io/crates/l/sova-rate-limit?style=for-the-badge)](https://github.com/s00d/sova/blob/master/LICENSE)
[![Donate](https://img.shields.io/badge/Donate-Donationalerts-ff4081?style=for-the-badge)](https://www.donationalerts.com/r/s00d88)

# sova-rate-limit

Rate limit plugin for Sova (local sliding + shared fixed window).

Part of [Sova](https://crates.io/crates/sova) — Express-like HTTP for Rust.

**Guide:** [https://s00d.github.io/sova/plugins/rate-limit](https://s00d.github.io/sova/plugins/rate-limit)  
**API:** [https://docs.rs/sova-rate-limit](https://docs.rs/sova-rate-limit)

## Install

Via the facade (recommended):

```bash
cargo add sova --features rate-limit
```

Or direct:

```bash
cargo add sova-rate-limit
```

## License

MIT — see [LICENSE](LICENSE).
