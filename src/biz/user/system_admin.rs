use app_error::AppError;
use sqlx::PgPool;
use uuid::Uuid;

pub async fn is_system_admin(pg_pool: &PgPool, user_uuid: &Uuid) -> Result<bool, AppError> {
  Ok(
    sqlx::query_scalar::<_, bool>("SELECT public.af_is_system_admin($1)")
      .bind(user_uuid)
      .fetch_one(pg_pool)
      .await?,
  )
}

pub fn app_metadata_is_system_admin(app_metadata: &serde_json::Value) -> bool {
  app_metadata
    .get("is_system_admin")
    .and_then(serde_json::Value::as_bool)
    .unwrap_or(false)
}

#[cfg(test)]
mod tests {
  use super::app_metadata_is_system_admin;

  #[test]
  fn reads_system_admin_flag() {
    assert!(app_metadata_is_system_admin(
      &serde_json::json!({ "is_system_admin": true })
    ));
    assert!(!app_metadata_is_system_admin(
      &serde_json::json!({ "is_system_admin": false })
    ));
    assert!(!app_metadata_is_system_admin(&serde_json::json!({})));
  }
}
