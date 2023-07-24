mod api;
mod database;
mod mihomo;
mod update;

use actix_files::Files;
use actix_session::{storage::CookieSessionStore, SessionMiddleware};
use std::{collections::HashMap, fs, sync::Mutex};

use actix_cors::Cors;
use actix_web::{cookie::Key, web::Data, App, HttpServer};
use sqlx::PgPool;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use uuid::Uuid;

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

#[derive(OpenApi)]
#[openapi(
    paths(
        api::mihomo::get_mihomo,
        api::scores::get_scores_achievement,
        api::scores::get_score_achievement,
        api::scores::put_score_achievement,
        api::scores::damage::get_scores_damage,
        api::scores::damage::get_score_damage,
        api::scores::damage::put_score_damage,
        api::scores::heal::get_scores_heal,
        api::scores::heal::get_score_heal,
        api::scores::heal::put_score_heal,
        api::scores::shield::get_scores_shield,
        api::scores::shield::get_score_shield,
        api::scores::shield::put_score_shield,
        api::users::login,
        api::users::register,
        api::users::logout,
        api::users::request_token,
        api::users::get_me,
        api::users::put_email,
        api::users::put_password,
        api::users::delete_email,
        api::users::get_verifications,
        api::users::put_verification,
        api::users::get_user_achievements,
        api::users::put_user_achievement,
        api::users::delete_user_achievement,
        api::achievements::put_achievement_reference,
        api::achievements::put_achievement_difficulty,
        api::achievements::delete_achievement_reference,
        api::achievements::delete_achievement_difficulty,
        api::submissions::damage::get_submissions_damage,
        api::submissions::damage::get_submission_damage,
        api::submissions::damage::post_submission_damage,
        api::submissions::damage::delete_submission_damage,
        api::submissions::heal::get_submissions_heal,
        api::submissions::heal::get_submission_heal,
        api::submissions::heal::post_submission_heal,
        api::submissions::heal::delete_submission_heal,
        api::submissions::shield::get_submissions_shield,
        api::submissions::shield::get_submission_shield,
        api::submissions::shield::post_submission_shield,
        api::submissions::shield::delete_submission_shield,
        api::import::import,
    ),
    components(schemas(
        api::schemas::Region,
        api::schemas::CharacterDamage,
        api::schemas::ScoreAchievement,
        api::schemas::ScoresAchievement,
        api::schemas::ScoreDamage,
        api::schemas::ScoresDamage,
        api::schemas::ScoreHeal,
        api::schemas::ScoresHeal,
        api::schemas::ScoreShield,
        api::schemas::ScoresShield,
        api::schemas::SubmissionDamage,
        api::schemas::SubmissionDamageUpdate,
        api::schemas::SubmissionHeal,
        api::schemas::SubmissionHealUpdate,
        api::schemas::SubmissionShield,
        api::schemas::SubmissionShieldUpdate,
        api::schemas::DamageUpdate,
        api::schemas::HealUpdate,
        api::schemas::ShieldUpdate,
        api::users::User,
        api::users::UserLogin,
        api::users::UserRegister,
        api::users::EmailUpdate,
        api::users::PasswordUpdate,
        api::users::RequestToken,
        api::users::Verification,
        api::users::Otp,
        api::achievements::ReferenceUpdate,
        api::achievements::DifficultyUpdate,
        api::import::File
    ))
)]
struct ApiDoc;

#[actix_web::main]
async fn main() -> Result<()> {
    env_logger::init();

    let _ = fs::create_dir("mihomo");

    let pool = PgPool::connect(dotenv_codegen::dotenv!("DATABASE_URL")).await?;

    sqlx::migrate!().run(&pool).await?;

    update::achievements(pool.clone()).await;
    update::verifications(pool.clone()).await;
    update::scores().await;

    let password_resets = Data::new(Mutex::new(HashMap::<Uuid, String>::new()));
    let pool = Data::new(pool);

    let key = Key::generate();

    let mut openapi = ApiDoc::openapi();
    openapi.merge(api::achievements::openapi());

    HttpServer::new(move || {
        App::new()
            .app_data(password_resets.clone())
            .app_data(pool.clone())
            .wrap(Cors::permissive())
            .wrap(SessionMiddleware::new(
                CookieSessionStore::default(),
                key.clone(),
            ))
            .service(Files::new("/static", "static").show_files_listing())
            .service(
                SwaggerUi::new("/swagger-ui/{_:.*}").url("/api-doc/openapi.json", openapi.clone()),
            )
            .configure(api::achievements::configure)
            .service(api::mihomo::get_mihomo)
            .service(api::scores::damage::get_scores_damage)
            .service(api::scores::damage::get_score_damage)
            .service(api::scores::damage::put_score_damage)
            .service(api::scores::heal::get_scores_heal)
            .service(api::scores::heal::get_score_heal)
            .service(api::scores::heal::put_score_heal)
            .service(api::scores::shield::get_scores_shield)
            .service(api::scores::shield::get_score_shield)
            .service(api::scores::shield::put_score_shield)
            .service(api::scores::get_scores_achievement)
            .service(api::scores::get_score_achievement)
            .service(api::scores::put_score_achievement)
            .service(api::mihomo::get_mihomo)
            .service(api::users::register)
            .service(api::users::login)
            .service(api::users::logout)
            .service(api::users::request_token)
            .service(api::users::get_me)
            .service(api::users::put_email)
            .service(api::users::put_password)
            .service(api::users::delete_email)
            .service(api::users::get_verifications)
            .service(api::users::put_verification)
            .service(api::users::get_user_achievements)
            .service(api::users::put_user_achievement)
            .service(api::users::delete_user_achievement)
            .service(api::achievements::put_achievement_reference)
            .service(api::achievements::put_achievement_difficulty)
            .service(api::achievements::delete_achievement_reference)
            .service(api::achievements::delete_achievement_difficulty)
            .service(api::submissions::damage::get_submissions_damage)
            .service(api::submissions::damage::get_submission_damage)
            .service(api::submissions::damage::post_submission_damage)
            .service(api::submissions::damage::delete_submission_damage)
            .service(api::submissions::heal::get_submissions_heal)
            .service(api::submissions::heal::get_submission_heal)
            .service(api::submissions::heal::post_submission_heal)
            .service(api::submissions::heal::delete_submission_heal)
            .service(api::submissions::shield::get_submissions_shield)
            .service(api::submissions::shield::get_submission_shield)
            .service(api::submissions::shield::post_submission_shield)
            .service(api::submissions::shield::delete_submission_shield)
            .service(api::import::import)
    })
    .bind(("localhost", 8000))?
    .run()
    .await?;

    Ok(())
}
