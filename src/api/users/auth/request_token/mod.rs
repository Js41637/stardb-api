use std::collections::HashMap;

use actix_web::{post, rt, web, HttpResponse, Responder};
use futures::lock::Mutex;
use lettre::{transport::smtp::authentication::Credentials, Message, SmtpTransport, Transport};
use serde::Deserialize;
use sqlx::PgPool;
use utoipa::{OpenApi, ToSchema};
use uuid::Uuid;

use crate::{database, Result};

#[derive(OpenApi)]
#[openapi(
    tags((name = "users/auth/request-token")),
    paths(request_token),
    components(schemas(RequestToken))
)]
struct ApiDoc;

pub fn openapi() -> utoipa::openapi::OpenApi {
    ApiDoc::openapi()
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(request_token);
}

#[derive(Deserialize, ToSchema)]
pub struct RequestToken {
    username: String,
}

#[utoipa::path(
    tag = "users/auth/request-token",
    post,
    path = "/api/users/auth/request-token",
    request_body = RequestToken,
    responses(
        (status = 200, description = "Send mail with emergency login"),
        (status = 400, description = "No email connected"),
    )
)]
#[post("/api/users/auth/request-token")]
async fn request_token(
    request_token: web::Json<RequestToken>,
    tokens: web::Data<Mutex<HashMap<Uuid, String>>>,
    pool: web::Data<PgPool>,
) -> Result<impl Responder> {
    {
        if tokens
            .lock()
            .await
            .values()
            .any(|s| s == &request_token.username)
        {
            return Ok(HttpResponse::BadRequest().finish());
        }
    }

    let Ok(user) = database::get_user_by_username(&request_token.username, &pool).await else {
        return Ok(HttpResponse::BadRequest().finish());
    };

    let Some(email) = user.email else {
        return Ok(HttpResponse::BadRequest().finish());
    };

    let to = format!("<{email}>").parse()?;

    let token = Uuid::new_v4();
    let email = Message::builder()
        .from("Julius Kreutz <noreply@kreutz.dev>".parse()?)
        .to(to)
        .subject("Stardb Password Reset")
        .body(format!("https://stardb.gg/login?token={token}"))?;

    let credentials =
        Credentials::new(dotenv::var("SMTP_USERNAME")?, dotenv::var("SMTP_PASSWORD")?);

    let mailer = SmtpTransport::relay("mail.hosting.de")?
        .credentials(credentials)
        .build();

    mailer.send(&email)?;

    tokens.lock().await.insert(token, user.username.clone());

    rt::spawn(async move {
        rt::time::sleep(std::time::Duration::from_secs(5 * 60)).await;

        tokens.lock().await.remove(&token);

        Result::<()>::Ok(())
    });

    Ok(HttpResponse::Ok().finish())
}
