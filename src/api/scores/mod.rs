use actix_web::{get, put, web, HttpResponse, Responder};
use chrono::{Duration, NaiveDateTime, Utc};
use regex::{Captures, Regex};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use strum::{Display, EnumString};
use utoipa::{IntoParams, ToSchema};

use crate::database::{self, DbScore};
use crate::{mihomo, Result};

pub mod damage;
pub mod heal;
pub mod shield;

#[derive(Display, EnumString, Serialize, Deserialize, ToSchema)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum Region {
    NA,
    EU,
    Asia,
    CN,
}

#[derive(Deserialize, IntoParams)]
struct ScoresParams {
    region: Option<Region>,
    query: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ScoresAchievement {
    count: i64,
    scores: Vec<ScoreAchievement>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ScoreAchievement {
    global_rank: i64,
    regional_rank: i64,
    uid: i64,
    region: Region,
    name: String,
    level: i32,
    avatar_icon: String,
    signature: String,
    character_count: i32,
    achievement_count: i32,
    character_name: String,
    character_icon: String,
    path_icon: String,
    element_color: String,
    element_icon: String,
    timestamp: NaiveDateTime,
}

impl<T: AsRef<DbScore>> From<T> for ScoreAchievement {
    fn from(value: T) -> Self {
        let db_score = value.as_ref();

        ScoreAchievement {
            global_rank: db_score.global_rank.unwrap(),
            regional_rank: db_score.regional_rank.unwrap(),
            uid: db_score.uid,
            region: db_score.region.parse().unwrap(),
            name: db_score.name.clone(),
            level: db_score.level,
            avatar_icon: db_score.avatar_icon.clone(),
            signature: db_score.signature.clone(),
            character_count: db_score.character_count,
            achievement_count: db_score.achievement_count,
            character_name: db_score.character_name.clone(),
            character_icon: db_score.character_icon.clone(),
            path_icon: db_score.path_icon.clone(),
            element_color: db_score.element_color.clone(),
            element_icon: db_score.element_icon.clone(),
            timestamp: db_score.timestamp,
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/scores",
    params(
        ScoresParams
    ),
    responses(
        (status = 200, description = "ScoresAchievement", body = ScoresAchievement),
    )
)]
#[get("/api/scores")]
async fn get_scores_achievement(
    scores_params: web::Query<ScoresParams>,
    pool: web::Data<PgPool>,
) -> Result<impl Responder> {
    let count = database::count_scores(&pool).await?;

    let db_scores = database::get_scores(
        scores_params.region.as_ref().map(|r| r.to_string()),
        scores_params.query.clone(),
        scores_params.limit,
        scores_params.offset,
        &pool,
    )
    .await?;

    let scores = db_scores.iter().map(ScoreAchievement::from).collect();

    let scores_achievement = ScoresAchievement { count, scores };

    Ok(HttpResponse::Ok().json(scores_achievement))
}

#[utoipa::path(
    get,
    path = "/api/scores/{uid}",
    responses(
        (status = 200, description = "ScoreAchievement", body = ScoreAchievement),
    )
)]
#[get("/api/scores/{uid}")]
async fn get_score_achievement(
    uid: web::Path<i64>,
    pool: web::Data<PgPool>,
) -> Result<impl Responder> {
    let score: ScoreAchievement = database::get_score_by_uid(*uid, &pool).await?.into();

    Ok(HttpResponse::Ok().json(score))
}

#[utoipa::path(
    put,
    path = "/api/scores/{uid}",
    responses(
        (status = 200, description = "ScoreAchievement updated", body = ScoreAchievement),
    )
)]
#[put("/api/scores/{uid}")]
async fn put_score_achievement(
    uid: web::Path<i64>,
    pool: web::Data<PgPool>,
) -> Result<impl Responder> {
    let now = Utc::now().naive_utc();

    let uid = *uid;

    let api_data = mihomo::get(uid).await?;

    let re = Regex::new(r"<[^>]*>")?;

    let name = re
        .replace_all(&api_data.player.nickname, |_: &Captures| "")
        .to_string();
    let region = match uid.to_string().chars().next() {
        Some('6') => "na",
        Some('7') => "eu",
        Some('8') | Some('9') => "asia",
        _ => "cn",
    }
    .to_string();
    let level = api_data.player.level;
    let avatar_icon = api_data.player.avatar.icon.clone();
    let signature = re
        .replace_all(&api_data.player.signature, |_: &Captures| "")
        .to_string();
    let character_count = api_data.player.space_info.avatar_count;
    let achievement_count = api_data.player.space_info.achievement_count;
    let character_name = api_data.characters[0].name.clone();
    let character_icon = api_data.characters[0].icon.clone();
    let path_icon = api_data.characters[0].path.icon.clone();
    let element_color = api_data.characters[0].element.color.clone();
    let element_icon = api_data.characters[0].element.icon.clone();
    let timestamp = database::get_score_by_uid(uid, &pool)
        .await
        .ok()
        .and_then(|sd| {
            if sd.achievement_count == achievement_count {
                Some(sd.timestamp)
            } else {
                None
            }
        })
        .unwrap_or(
            now + match region.as_str() {
                "na" => Duration::hours(-5),
                "eu" => Duration::hours(1),
                _ => Duration::hours(8),
            },
        );

    let db_score = DbScore {
        uid,
        region,
        name,
        level,
        avatar_icon,
        signature,
        character_count,
        achievement_count,
        character_name,
        character_icon,
        path_icon,
        element_color,
        element_icon,
        timestamp,
        ..Default::default()
    };

    let score: ScoreAchievement = database::set_score(&db_score, &pool).await?.into();

    Ok(HttpResponse::Ok().json(score))
}
