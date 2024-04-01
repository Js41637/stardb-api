use actix_web::{get, put, web, HttpResponse, Responder};
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use sqlx::PgPool;
use utoipa::OpenApi;

use crate::{
    api::{private, ApiResult, LanguageParams, Region},
    database, mihomo, Language,
};

#[derive(OpenApi)]
#[openapi(paths(get_profile, update_profile))]
struct ApiDoc;

pub fn openapi() -> utoipa::openapi::OpenApi {
    ApiDoc::openapi()
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(get_profile).service(update_profile);
}

#[derive(Serialize)]
struct Profile {
    rank_global: i64,
    rank_regional: i64,
    top_global: f64,
    top_regional: f64,
    region: Region,
    updated_at: DateTime<Utc>,
    mihomo: Value,
}

#[utoipa::path(
    tag = "pages",
    get,
    path = "/api/pages/profiles/{uid}",
    params(LanguageParams),
    security(("api_key" = [])),
    responses(
        (status = 200, description = "Profile"),
    )
)]
#[get("/api/pages/profiles/{uid}", guard = "private")]
async fn get_profile(
    uid: web::Path<i64>,
    language_params: web::Query<LanguageParams>,
    pool: web::Data<PgPool>,
) -> ApiResult<impl Responder> {
    let profile = get_profile_json(*uid, language_params.lang, &pool).await?;

    Ok(HttpResponse::Ok().json(profile))
}

#[utoipa::path(
    tag = "pages",
    put,
    path = "/api/pages/profiles/{uid}",
    params(LanguageParams),
    security(("api_key" = [])),
    responses(
        (status = 200, description = "Profile"),
    )
)]
#[put("/api/pages/profiles/{uid}", guard = "private")]
async fn update_profile(
    uid: web::Path<i64>,
    language_params: web::Query<LanguageParams>,
    pool: web::Data<PgPool>,
) -> ApiResult<impl Responder> {
    reqwest::Client::new()
        .put(format!("http://localhost:8000/api/mihomo/{uid}"))
        .send()
        .await?;

    let profile = get_profile_json(*uid, language_params.lang, &pool).await?;

    Ok(HttpResponse::Ok().json(profile))
}

async fn get_profile_json(uid: i64, lang: Language, pool: &PgPool) -> ApiResult<Profile> {
    let mihomo = mihomo::get_whole(uid, lang).await?;

    let score_achievement = database::get_score_achievement_by_uid(uid, pool).await?;

    let rank_global = score_achievement.global_rank.unwrap_or_default();
    let rank_regional = score_achievement.regional_rank.unwrap_or_default();

    let count_global = database::count_scores_achievement(None, None, pool).await?;
    let count_regional =
        database::count_scores_achievement(Some(&score_achievement.region), None, pool).await?;

    let top_global = rank_global as f64 / count_global as f64;
    let top_regional = rank_regional as f64 / count_regional as f64;

    let region = score_achievement.region.parse()?;

    let updated_at = score_achievement.updated_at;

    let profile = Profile {
        rank_global,
        rank_regional,
        top_global,
        top_regional,
        updated_at,
        region,
        mihomo,
    };

    Ok(profile)
}
