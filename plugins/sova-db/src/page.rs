//! Offset pagination helpers (`Page` / `PageParams` / SeaORM `paginate`).

use crate::error::DbError;
use crate::handle::DbHandle;
use sova_core::Request;
use sea_orm::{EntityTrait, FromQueryResult, PaginatorTrait, Select};
use serde::Serialize;

const DEFAULT_PER_PAGE: u64 = 15;
const MAX_PER_PAGE: u64 = 100;

/// 1-based page + page size from query (`?page=&per_page=`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageParams {
    pub page: u64,
    pub per_page: u64,
}

impl Default for PageParams {
    fn default() -> Self {
        Self {
            page: 1,
            per_page: DEFAULT_PER_PAGE,
        }
    }
}

impl PageParams {
    pub fn new(page: u64, per_page: u64) -> Self {
        Self {
            page: page.max(1),
            per_page: per_page.clamp(1, MAX_PER_PAGE),
        }
    }

    pub fn from_request(req: &Request) -> Self {
        let page = req
            .query("page")
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);
        let per_page = req
            .query("per_page")
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_PER_PAGE);
        Self::new(page, per_page)
    }

    pub fn offset(&self) -> u64 {
        self.page.saturating_sub(1).saturating_mul(self.per_page)
    }
}

/// One page of results + meta.
#[derive(Debug, Clone, Serialize)]
pub struct Page<T> {
    pub data: Vec<T>,
    pub total: u64,
    pub page: u64,
    pub per_page: u64,
    pub last_page: u64,
}

impl<T> Page<T> {
    pub fn has_more(&self) -> bool {
        self.page < self.last_page
    }
}

/// `req.page_params()`.
pub trait PageExt {
    fn page_params(&self) -> PageParams;
}

impl PageExt for Request {
    fn page_params(&self) -> PageParams {
        PageParams::from_request(self)
    }
}

/// Paginate a SeaORM [`Select`].
pub trait PaginateExt<E>
where
    E: EntityTrait,
{
    fn paginate_sova(
        self,
        db: &DbHandle,
        params: PageParams,
    ) -> impl std::future::Future<Output = Result<Page<E::Model>, DbError>> + Send;
}

impl<E> PaginateExt<E> for Select<E>
where
    E: EntityTrait + Send,
    E::Model: FromQueryResult + Sized + Send + Sync,
{
    async fn paginate_sova(
        self,
        db: &DbHandle,
        params: PageParams,
    ) -> Result<Page<E::Model>, DbError> {
        let params = PageParams::new(params.page, params.per_page);
        let paginator = self.paginate(db, params.per_page);
        let total = paginator.num_items().await.map_err(DbError::from)?;
        let last_page = if total == 0 {
            1
        } else {
            total.div_ceil(params.per_page).max(1)
        };
        let page = params.page.min(last_page);
        let data = paginator
            .fetch_page(page.saturating_sub(1))
            .await
            .map_err(DbError::from)?;
        Ok(Page {
            data,
            total,
            page,
            per_page: params.per_page,
            last_page,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sova_core::Request;

    #[test]
    fn page_params_from_query() {
        let req = Request::builder()
            .path("/notes")
            .query_param("page", "3")
            .query_param("per_page", "10")
            .build();
        let p = PageParams::from_request(&req);
        assert_eq!(p.page, 3);
        assert_eq!(p.per_page, 10);
        assert_eq!(p.offset(), 20);
    }

    #[test]
    fn page_params_clamp() {
        let p = PageParams::new(0, 999);
        assert_eq!(p.page, 1);
        assert_eq!(p.per_page, 100);
    }
}
