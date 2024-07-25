mod uid;

use std::{collections::HashMap, sync::Arc, time::Duration};

use actix_session::Session;
use actix_web::{post, rt, web, HttpResponse, Responder};
use chrono::NaiveDateTime;
use futures::lock::Mutex;
use reqwest::header;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use strum::IntoEnumIterator;
use url::Url;
use utoipa::{OpenApi, ToSchema};

use crate::{api::ApiResult, database, GiGachaType};

#[derive(OpenApi)]
#[openapi(
    tags((name = "gi/wishes-import")),
    paths(post_gi_wishes_import),
    components(schemas(WishesImportParams, WishesImport, WishesImportInfo, Status))
)]
struct ApiDoc;

pub fn openapi() -> utoipa::openapi::OpenApi {
    let mut openapi = ApiDoc::openapi();
    openapi.merge(uid::openapi());
    openapi
}

lazy_static::lazy_static! {
    static ref DATA: web::Data<WishesImportInfos> = web::Data::new(WishesImportInfos::default());
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.app_data(DATA.clone())
        .service(post_gi_wishes_import)
        .configure(uid::configure);
}

#[derive(Deserialize)]
struct GachaLog {
    data: Data,
}

#[derive(Deserialize)]
struct Data {
    list: Vec<Entry>,
    region: String,
}

#[derive(Deserialize)]
struct Entry {
    id: String,
    uid: String,
    item_type: String,
    name: String,
    time: String,
}

type WishesImportInfos = Mutex<HashMap<i32, Arc<Mutex<WishesImportInfo>>>>;

#[derive(Serialize, ToSchema, Clone)]
#[serde(rename_all = "snake_case")]
enum Status {
    Pending,
    Calculating,
    Finished,
    Error(String),
}

#[derive(Serialize, ToSchema, Clone)]
struct WishesImportInfo {
    gacha_type: GiGachaType,
    beginner: usize,
    standard: usize,
    character: usize,
    weapon: usize,
    chronicled: usize,
    status: Status,
}

#[derive(Deserialize, ToSchema)]
struct WishesImportParams {
    url: String,
}

#[derive(Serialize, ToSchema)]
struct WishesImport {
    uid: i32,
}

#[utoipa::path(
    tag = "gi/wishes-import",
    post,
    path = "/api/gi/wishes-import",
    request_body = WishesImportParams,
    responses(
        (status = 200, description = "WishesImport", body = WishesImport),
    )
)]
#[post("/api/gi/wishes-import")]
async fn post_gi_wishes_import(
    session: Session,
    params: web::Json<WishesImportParams>,
    wishes_import_infos: web::Data<WishesImportInfos>,
    pool: web::Data<PgPool>,
) -> ApiResult<impl Responder> {
    let url = Url::parse(&params.url)?;

    let query = url.query_pairs().filter(|(name, _)| {
        matches!(
            name.to_string().as_str(),
            "authkey" | "authkey_ver" | "sign_type"
        )
    });

    let mut url =
        Url::parse("https://public-operation-hk4e-sg.hoyoverse.com/gacha_info/api/getGachaLog")?;

    url.query_pairs_mut()
        .extend_pairs(query)
        .extend_pairs(&[("lang", "en"), ("size", "20")])
        .finish();

    let mut uid = 0;

    for gacha_type in [100, 200, 301, 302, 500] {
        let gacha_log: GachaLog = reqwest::get(format!("{url}&gacha_type={gacha_type}&end_id=0"))
            .await?
            .json()
            .await?;

        if let Some(entry) = gacha_log.data.list.first() {
            uid = entry.uid.parse()?;
            break;
        }
    }

    if uid == 0 {
        let info = Arc::new(Mutex::new(WishesImportInfo {
            gacha_type: GiGachaType::Standard,
            beginner: 0,
            standard: 0,
            character: 0,
            weapon: 0,
            chronicled: 0,
            status: Status::Error("No data".to_string()),
        }));

        wishes_import_infos.lock().await.insert(uid, info.clone());

        return Ok(HttpResponse::Ok().json(WishesImport { uid }));
    }

    let name = reqwest::Client::new()
        .get(format!("https://enka.network/api/uid/{uid}?info"))
        .header(header::USER_AGENT, "stardb")
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?["playerInfo"]["nickname"]
        .as_str()
        .unwrap()
        .to_string();

    database::gi::profiles::set(&database::gi::profiles::DbProfile { uid, name }, &pool).await?;
    if let Ok(Some(username)) = session.get::<String>("username") {
        let connection = database::gi::connections::DbConnection {
            uid,
            username,
            verified: true,
            private: false,
        };

        database::gi::connections::set(&connection, &pool).await?;
    }

    if wishes_import_infos.lock().await.contains_key(&uid) {
        return Ok(HttpResponse::Ok().json(WishesImport { uid }));
    }

    let info = Arc::new(Mutex::new(WishesImportInfo {
        gacha_type: GiGachaType::Standard,
        beginner: 0,
        standard: 0,
        character: 0,
        weapon: 0,
        chronicled: 0,
        status: Status::Pending,
    }));

    wishes_import_infos.lock().await.insert(uid, info.clone());

    rt::spawn(async move {
        let mut error = Ok(());

        for gacha_type in GiGachaType::iter() {
            info.lock().await.gacha_type = gacha_type;

            if let Err(e) = import_wishes(&url, gacha_type, &info, &pool).await {
                error = Err(e);

                break;
            }
        }

        if let Err(e) = error {
            info.lock().await.status = Status::Error(e.to_string());
        } else if let Err(e) = calculate_stats(uid, &info, &pool).await {
            info.lock().await.status = Status::Error(e.to_string());
        } else {
            info.lock().await.status = Status::Finished;
        }

        rt::spawn(async move {
            rt::time::sleep(Duration::from_secs(60)).await;

            wishes_import_infos.lock().await.remove(&uid);
        });
    });

    Ok(HttpResponse::Ok().json(WishesImport { uid }))
}

async fn import_wishes(
    url: &Url,
    gacha_type: GiGachaType,
    info: &Arc<Mutex<WishesImportInfo>>,
    pool: &PgPool,
) -> ApiResult<()> {
    let mut url = url.clone();
    let mut end_id = "0".to_string();

    url.query_pairs_mut()
        .extend_pairs(&[(
            "gacha_type",
            match gacha_type {
                GiGachaType::Beginner => "100",
                GiGachaType::Standard => "200",
                GiGachaType::Character => "301",
                GiGachaType::Weapon => "302",
                GiGachaType::Chronicled => "500",
            },
        )])
        .finish();

    let mut set_all = database::gi::wishes::SetAll::default();

    loop {
        let mut i = 0;
        let gacha_log = loop {
            let response = reqwest::get(format!("{url}&end_id={end_id}")).await?;
            if let Ok(gacha_log) = response.json::<GachaLog>().await {
                break gacha_log;
            }

            if i > 2 {
                return Err(anyhow::anyhow!("Unsure").into());
            }

            rt::time::sleep(Duration::from_secs(1)).await;

            i += 1;
        };

        if gacha_log.data.list.is_empty() {
            break;
        }

        let region_time_zone = match gacha_log.data.region.as_str() {
            "os_usa" => -5,
            "os_eu" => 1,
            _ => 8,
        };

        let timestamp_offset = chrono::Duration::hours(region_time_zone);

        for entry in gacha_log.data.list {
            end_id.clone_from(&entry.id);

            let id = entry.id.parse()?;
            let uid: i32 = entry.uid.parse()?;

            let exists = match gacha_type {
                GiGachaType::Beginner => {
                    database::gi::wishes::beginner::exists(id, uid, pool).await?
                }
                GiGachaType::Standard => {
                    database::gi::wishes::standard::exists(id, uid, pool).await?
                }
                GiGachaType::Character => {
                    database::gi::wishes::character::exists(id, uid, pool).await?
                }
                GiGachaType::Weapon => database::gi::wishes::weapon::exists(id, uid, pool).await?,
                GiGachaType::Chronicled => {
                    database::gi::wishes::chronicled::exists(id, uid, pool).await?
                }
            };

            if exists {
                continue;
            }

            let item: i32 = if let Ok(id) =
                database::gi::characters_text::get_id_by_name(&entry.name, pool).await
            {
                id
            } else {
                database::gi::weapons_text::get_id_by_name(&entry.name, pool).await?
            };

            let mut character = (entry.item_type == "Character").then_some(item);
            let mut weapon = (entry.item_type == "Weapon").then_some(item);

            if character.is_none() && weapon.is_none() {
                if item >= 10000000 {
                    character = Some(item);
                } else {
                    weapon = Some(item);
                }
            }

            let timestamp = NaiveDateTime::parse_from_str(&entry.time, "%Y-%m-%d %H:%M:%S")?
                .and_utc()
                - timestamp_offset;

            set_all.id.push(id);
            set_all.uid.push(uid);
            set_all.character.push(character);
            set_all.weapon.push(weapon);
            set_all.timestamp.push(timestamp);
            set_all.official.push(true);

            match gacha_type {
                GiGachaType::Beginner => info.lock().await.beginner += 1,
                GiGachaType::Standard => info.lock().await.standard += 1,
                GiGachaType::Character => info.lock().await.character += 1,
                GiGachaType::Weapon => info.lock().await.weapon += 1,
                GiGachaType::Chronicled => info.lock().await.chronicled += 1,
            }
        }
    }

    match gacha_type {
        GiGachaType::Beginner => database::gi::wishes::beginner::set_all(&set_all, pool).await?,
        GiGachaType::Standard => database::gi::wishes::standard::set_all(&set_all, pool).await?,
        GiGachaType::Character => database::gi::wishes::character::set_all(&set_all, pool).await?,
        GiGachaType::Weapon => database::gi::wishes::weapon::set_all(&set_all, pool).await?,
        GiGachaType::Chronicled => {
            database::gi::wishes::chronicled::set_all(&set_all, pool).await?
        }
    }

    Ok(())
}

async fn calculate_stats(
    uid: i32,
    info: &Arc<Mutex<WishesImportInfo>>,
    pool: &PgPool,
) -> anyhow::Result<()> {
    info.lock().await.status = Status::Calculating;

    Ok(())
}