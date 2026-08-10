//! Connect-JSON unary RPC for Sova — client first, optional server.
//!
//! ```ignore
//! use sova_grpc::{FakeGrpc, Grpc, GrpcExt};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Serialize)]
//! struct HelloIn { name: String }
//! #[derive(Deserialize)]
//! struct HelloOut { message: String }
//!
//! let fake = FakeGrpc::new().stub_json("hello.Greeter/SayHello", serde_json::json!({
//!     "message": "hi"
//! }));
//! app.install(Grpc::fake(fake));
//!
//! let out: HelloOut = req.grpc().call("hello.Greeter/SayHello", &HelloIn { name: "a".into() }).await?;
//! ```

mod bound;
mod client;
mod error;
mod error_envelope;
mod fake;
mod plugin;
mod router;
mod server;
mod trace;
mod transport;

pub use bound::{GrpcBound, GrpcExt};
pub use client::GrpcClient;
pub use error::GrpcError;
pub use fake::{FakeGrpc, GrpcCall};
pub use plugin::{Grpc, GrpcServerBuilder};
pub use router::MethodRouter;
pub use transport::GrpcTransport;
