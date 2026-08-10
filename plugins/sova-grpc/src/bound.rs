//! `req.grpc()`.

use crate::client::GrpcClient;
use crate::error::GrpcError;
use bytes::Bytes;
use serde::{de::DeserializeOwned, Serialize};
use sova_core::Request;

pub trait GrpcExt {
    fn grpc(&self) -> GrpcBound;
    fn try_grpc(&self) -> Option<GrpcBound>;
}

impl GrpcExt for Request {
    fn grpc(&self) -> GrpcBound {
        GrpcBound {
            client: self.state::<GrpcClient>(),
        }
    }

    fn try_grpc(&self) -> Option<GrpcBound> {
        self.try_state::<GrpcClient>()
            .map(|client| GrpcBound { client })
    }
}

pub struct GrpcBound {
    client: std::sync::Arc<GrpcClient>,
}

impl GrpcBound {
    pub fn client(&self) -> &GrpcClient {
        &self.client
    }

    pub async fn call<Req, Res>(&self, method: &str, req: &Req) -> Result<Res, GrpcError>
    where
        Req: Serialize,
        Res: DeserializeOwned,
    {
        self.client.call(method, req).await
    }

    pub async fn call_raw(&self, method: &str, body: Bytes) -> Result<Bytes, GrpcError> {
        self.client.call_raw(method, body).await
    }
}
