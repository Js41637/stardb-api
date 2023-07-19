use sqlx::PgPool;

use crate::Result;

pub struct DbVerification {
    pub uid: i64,
    pub username: String,
    pub otp: String,
}

impl AsRef<DbVerification> for DbVerification {
    fn as_ref(&self) -> &DbVerification {
        self
    }
}

pub async fn set_verification(verification: &DbVerification, pool: &PgPool) -> Result<()> {
    sqlx::query!(
        "
        INSERT INTO
            verifications(uid, username, otp)
        VALUES
            ($1, $2, $3)
        ON CONFLICT
            (uid)
        DO UPDATE SET
            username = EXCLUDED.username,
            otp = EXCLUDED.otp
        ",
        verification.uid,
        verification.username,
        verification.otp,
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_verifications(pool: &PgPool) -> Result<Vec<DbVerification>> {
    Ok(
        sqlx::query_as!(DbVerification, "SELECT * FROM verifications")
            .fetch_all(pool)
            .await?,
    )
}

pub async fn get_verifications_by_username(
    username: &str,
    pool: &PgPool,
) -> Result<Vec<DbVerification>> {
    Ok(sqlx::query_as!(
        DbVerification,
        "SELECT * FROM verifications WHERE username = $1",
        username
    )
    .fetch_all(pool)
    .await?)
}

pub async fn delete_verification_by_uid(uid: i64, pool: &PgPool) -> Result<()> {
    sqlx::query!("DELETE FROM verifications WHERE uid = $1", uid)
        .execute(pool)
        .await?;

    Ok(())
}
