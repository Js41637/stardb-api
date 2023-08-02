use actix_session::Session;
use actix_web::{delete, put, web, HttpResponse, Responder};
use sqlx::PgPool;
use utoipa::OpenApi;

use crate::{database, Result};

#[derive(OpenApi)]
#[openapi(
    tags((name = "achievements/{id}/gacha")),
    paths(put_achievement_gacha, delete_achievement_gacha)
)]
struct ApiDoc;

pub fn openapi() -> utoipa::openapi::OpenApi {
    ApiDoc::openapi()
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(put_achievement_gacha)
        .service(delete_achievement_gacha);
}

#[utoipa::path(
    tag = "achievements/{id}/gacha",
    put,
    path = "/api/achievements/{id}/gacha",
    responses(
        (status = 200, description = "Updated gacha"),
        (status = 403, description = "Not an admin"),
    ),
    security(("admin" = []))
)]
#[put("/api/achievements/{id}/gacha")]
async fn put_achievement_gacha(
    session: Session,
    id: web::Path<i64>,
    pool: web::Data<PgPool>,
) -> Result<impl Responder> {
    let Ok(Some(admin)) = session.get::<bool>("admin") else {
        return Ok(HttpResponse::BadRequest().finish());
    };

    if !admin {
        return Ok(HttpResponse::Forbidden().finish());
    }

    database::update_achievement_gacha(*id, true, &pool).await?;

    Ok(HttpResponse::Ok().finish())
}

#[utoipa::path(
    tag = "achievements/{id}/gacha",
    delete,
    path = "/api/achievements/{id}/gacha",
    responses(
        (status = 200, description = "Deleted gacha"),
        (status = 403, description = "Not an admin"),
    ),
    security(("admin" = []))
)]
#[delete("/api/achievements/{id}/gacha")]
async fn delete_achievement_gacha(
    session: Session,
    id: web::Path<i64>,
    pool: web::Data<PgPool>,
) -> Result<impl Responder> {
    let Ok(Some(admin)) = session.get::<bool>("admin") else {
        return Ok(HttpResponse::BadRequest().finish());
    };

    if !admin {
        return Ok(HttpResponse::Forbidden().finish());
    }

    database::update_achievement_gacha(*id, false, &pool).await?;

    Ok(HttpResponse::Ok().finish())
}
