use std::{fs::File, path::PathBuf};

use actix_web::{get, web, HttpResponse, Responder};
use serde_json::Value;

use crate::Result;

#[utoipa::path(
    get,
    path = "/api/mihomo/{uid}",
    responses(
        (status = 200, description = "Cached mihomo json"),
    )
)]
#[get("/api/mihomo/{uid}")]
async fn get_mihomo(uid: web::Path<i64>) -> Result<impl Responder> {
    let path = format!("mihomo/{uid}.json");

    if !PathBuf::from(&path).exists() {
        reqwest::Client::new()
            .put(&format!("http://localhost:8000/api/scores/{uid}"))
            .send()
            .await?;
    }

    let json: Value = serde_json::from_reader(File::open(&path)?)?;

    Ok(HttpResponse::Ok().json(json))
}
