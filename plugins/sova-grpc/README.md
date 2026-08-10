[![crates.io](https://img.shields.io/crates/v/sova-grpc?style=for-the-badge)](https://crates.io/crates/sova-grpc)
[![docs.rs](https://img.shields.io/docsrs/sova-grpc?style=for-the-badge)](https://docs.rs/sova-grpc)
[![License](https://img.shields.io/crates/l/sova-grpc?style=for-the-badge)](https://github.com/s00d/sova/blob/master/LICENSE)

# sova-grpc

Connect-JSON unary RPC for Sova — **client first** (`req.grpc().call`), `FakeGrpc`, optional unary server mount.

No tonic / `.proto` required for v1 (serde JSON).

**Guide:** [https://s00d.github.io/sova/plugins/grpc](https://s00d.github.io/sova/plugins/grpc)

## Install

```bash
cargo add sova --features grpc
```

Server mount: `Grpc::server().unary(...).client(url)`.

## License

MIT — see [LICENSE](LICENSE).
