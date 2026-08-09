use super::dto::{Create, IdParams, Update};
use crate::entities::post as entity;
use sova::{
    ActiveModelTrait, DbError, DbExt, Error, Json, Request, Response, Result, Set, ValidationExt,
};
use sea_orm::EntityTrait;

pub async fn list(req: Request) -> Result<Json<Vec<entity::Model>>> {
    let rows = entity::Entity::find()
        .all(req.db())
        .await
        .map_err(DbError::from)?;
    Ok(Json(rows))
}

pub async fn create(mut req: Request) -> Result<(u16, Json<entity::Model>)> {
    let body: Create = req.validate().await?;
    let row = entity::ActiveModel {
        title: Set(body.title),
        body: Set(body.body),
        ..Default::default()
    }
    .insert(req.db())
    .await
    .map_err(DbError::from)?;
    Ok((201, Json(row)))
}

pub async fn show(req: Request) -> Result<Json<entity::Model>> {
    let params: IdParams = req.validate_params()?;
    let id: i32 = params
        .id
        .parse()
        .map_err(|_| Error::BadRequest("invalid id".into()))?;
    let row = entity::Entity::find_by_id(id)
        .one(req.db())
        .await
        .map_err(DbError::from)?
        .ok_or(Error::NotFound)?;
    Ok(Json(row))
}

pub async fn update(mut req: Request) -> Result<Json<entity::Model>> {
    let params: IdParams = req.validate_params()?;
    let id: i32 = params
        .id
        .parse()
        .map_err(|_| Error::BadRequest("invalid id".into()))?;
    let body: Update = req.validate().await?;
    let row = entity::Entity::find_by_id(id)
        .one(req.db())
        .await
        .map_err(DbError::from)?
        .ok_or(Error::NotFound)?;
    let mut am: entity::ActiveModel = row.into();
    am.title = Set(body.title);
    am.body = Set(body.body);
    Ok(Json(am.update(req.db()).await.map_err(DbError::from)?))
}

pub async fn destroy(req: Request) -> Result<Response> {
    let params: IdParams = req.validate_params()?;
    let id: i32 = params
        .id
        .parse()
        .map_err(|_| Error::BadRequest("invalid id".into()))?;
    let row = entity::Entity::find_by_id(id)
        .one(req.db())
        .await
        .map_err(DbError::from)?
        .ok_or(Error::NotFound)?;
    let am: entity::ActiveModel = row.into();
    am.delete(req.db()).await.map_err(DbError::from)?;
    Ok(Response::empty().status(204))
}
