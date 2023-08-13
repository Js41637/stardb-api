mod grouped;
mod id;

use std::time::Duration;

use actix_web::{get, rt, web, HttpResponse, Responder};
use futures::lock::Mutex;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use strum::{Display, EnumString};
use utoipa::{OpenApi, ToSchema};

use crate::{
    database::{self, DbAchievement},
    Result,
};

#[derive(OpenApi)]
#[openapi(
    tags((name = "achievements")),
    paths(get_achievements),
    components(schemas(
        Difficulty,
        Achievement
    ))
)]
struct ApiDoc;

#[derive(Display, EnumString, Serialize, Deserialize, ToSchema)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
enum Difficulty {
    Easy,
    Medium,
    Hard,
}

#[derive(Serialize, ToSchema)]
struct Achievement {
    id: i64,
    series: i32,
    series_name: String,
    name: String,
    description: String,
    jades: i32,
    hidden: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reference: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    difficulty: Option<Difficulty>,
    gacha: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    set: Option<i32>,
    percent: f64,
}

impl From<DbAchievement> for Achievement {
    fn from(db_achievement: DbAchievement) -> Self {
        Achievement {
            id: db_achievement.id,
            series: db_achievement.series,
            series_name: db_achievement.series_name.clone(),
            name: db_achievement.name.clone(),
            description: db_achievement.description.clone(),
            jades: db_achievement.jades,
            hidden: db_achievement.hidden,
            version: db_achievement.version.clone(),
            comment: db_achievement.comment.clone(),
            reference: db_achievement.reference.clone(),
            difficulty: db_achievement
                .difficulty
                .as_ref()
                .map(|d| d.parse().unwrap()),
            gacha: db_achievement.gacha,
            set: db_achievement.set,
            percent: db_achievement.percent.unwrap_or_default(),
        }
    }
}

pub fn openapi() -> utoipa::openapi::OpenApi {
    let mut openapi = ApiDoc::openapi();
    openapi.merge(grouped::openapi());
    openapi.merge(id::openapi());
    openapi
}

pub fn configure(cfg: &mut web::ServiceConfig, pool: PgPool) {
    let achievements = web::Data::new(Mutex::new(Vec::new()));

    {
        let achievements = achievements.clone();
        let pool = pool.clone();

        rt::spawn(async move {
            let minutes = 1;

            let mut timer = rt::time::interval(Duration::from_secs(60 * minutes));

            loop {
                timer.tick().await;

                let _ = update(&achievements, &pool).await;
            }
        });
    }

    cfg.app_data(achievements)
        .service(get_achievements)
        .configure(|cfg| grouped::configure(cfg, pool))
        .configure(id::configure);
}

async fn update(achievements: &web::Data<Mutex<Vec<Achievement>>>, pool: &PgPool) -> Result<()> {
    let db_achievements = database::get_achievements(pool).await?;

    *achievements.lock().await = db_achievements
        .clone()
        .into_iter()
        .map(Achievement::from)
        .collect();

    Ok(())
}

#[utoipa::path(
    tag = "achievements",
    get,
    path = "/api/achievements",
    responses(
        (status = 200, description = "[Achievement]", body = Vec<Achievement>),
    )
)]
#[get("/api/achievements")]
async fn get_achievements(
    achievements: web::Data<Mutex<Vec<Achievement>>>,
) -> Result<impl Responder> {
    Ok(HttpResponse::Ok().json(&*achievements.lock().await))
}
