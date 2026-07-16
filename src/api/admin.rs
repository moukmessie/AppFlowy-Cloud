use actix_web::{web, Scope};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::biz::authentication::jwt::Authorization;
use crate::state::AppState;
use shared_entity::response::{AppResponse, JsonAppResponse};

#[derive(Debug, Serialize, FromRow)]
struct AdminUser {
  id: i64,
  uuid: Uuid,
  name: String,
  email: String,
  is_system_admin: bool,
}

#[derive(Debug, Deserialize)]
struct ListUsersQuery {
  search: Option<String>,
  limit: Option<i64>,
  offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct SetSystemAdmin {
  enabled: bool,
}

#[derive(Debug, Serialize, FromRow)]
struct SystemConfig {
  key: String,
  value: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct UpsertSystemConfig {
  value: serde_json::Value,
}

#[derive(Debug, Serialize, FromRow)]
struct SignupWhitelistEntry {
  id: Uuid,
  kind: String,
  value: String,
}

#[derive(Debug, Deserialize)]
struct CreateSignupWhitelistEntry {
  kind: String,
  value: String,
}

#[derive(Debug, Serialize, FromRow)]
struct PendingGuestInvitation {
  id: Uuid,
  workspace_id: Uuid,
  invitee_email: String,
  created_at: chrono::DateTime<chrono::Utc>,
}

pub fn admin_scope() -> Scope {
  web::scope("/api/admin")
    .service(web::resource("/users").route(web::get().to(list_users)))
    .service(
      web::resource("/users/{user_uuid}/system-admin")
        .route(web::put().to(set_system_admin)),
    )
    .service(web::resource("/system-config").route(web::get().to(list_system_config)))
    .service(
      web::resource("/system-config/{key}").route(web::put().to(upsert_system_config)),
    )
    .service(
      web::resource("/signup-whitelist")
        .route(web::get().to(list_signup_whitelist))
        .route(web::post().to(create_signup_whitelist)),
    )
    .service(
      web::resource("/signup-whitelist/{entry_id}")
        .route(web::delete().to(delete_signup_whitelist)),
    )
    .service(
      web::resource("/guests/pending-admin-approval")
        .route(web::get().to(list_pending_guest_invitations)),
    )
    .service(
      web::resource("/guests/pending-admin-approval/{invite_id}/approve")
        .route(web::post().to(approve_guest_invitation)),
    )
    .service(
      web::resource("/guests/pending-admin-approval/{invite_id}/reject")
        .route(web::post().to(reject_guest_invitation)),
    )
}

const SUPPORTED_SYSTEM_CONFIG: [&str; 2] = [
  "signup_whitelist_enabled",
  "guest_invites_require_admin_approval",
];

async fn require_system_admin(
  state: &AppState,
  auth: &Authorization,
) -> Result<Uuid, actix_web::Error> {
  let user_uuid = auth.uuid()?;
  let allowed = sqlx::query_scalar::<_, bool>("SELECT public.af_is_system_admin($1)")
    .bind(user_uuid)
    .fetch_one(&state.pg_pool)
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?;

  if !allowed {
    return Err(actix_web::error::ErrorForbidden(
      "System administrator privileges are required",
    ));
  }
  Ok(user_uuid)
}

async fn list_users(
  auth: Authorization,
  state: web::Data<AppState>,
  query: web::Query<ListUsersQuery>,
) -> Result<JsonAppResponse<Vec<AdminUser>>, actix_web::Error> {
  require_system_admin(&state, &auth).await?;
  let search = query.search.as_deref().unwrap_or("").trim();
  let limit = query.limit.unwrap_or(50).clamp(1, 200);
  let offset = query.offset.unwrap_or(0).max(0);

  let users = sqlx::query_as::<_, AdminUser>(
    r#"
      SELECT u.uid AS id, u.uuid, u.name, u.email,
             (COALESCE(au.is_super_admin, false)
              OR COALESCE((au.raw_app_meta_data->>'is_system_admin')::boolean, false))
               AS is_system_admin
        FROM af_user u
        LEFT JOIN auth.users au ON au.id = u.uuid
       WHERE ($1 = '' OR u.name ILIKE '%' || $1 || '%' OR u.email ILIKE '%' || $1 || '%')
       ORDER BY u.created_at ASC
       LIMIT $2 OFFSET $3
    "#,
  )
  .bind(search)
  .bind(limit)
  .bind(offset)
  .fetch_all(&state.pg_pool)
  .await
  .map_err(actix_web::error::ErrorInternalServerError)?;

  Ok(AppResponse::Ok().with_data(users).into())
}

async fn set_system_admin(
  auth: Authorization,
  state: web::Data<AppState>,
  user_uuid: web::Path<Uuid>,
  payload: web::Json<SetSystemAdmin>,
) -> Result<JsonAppResponse<()>, actix_web::Error> {
  let actor_uuid = require_system_admin(&state, &auth).await?;
  let target_uuid = user_uuid.into_inner();

  if !payload.enabled {
    let admin_count = sqlx::query_scalar::<_, i64>(
      r#"SELECT COUNT(*) FROM auth.users
          WHERE COALESCE(is_super_admin, false)
             OR COALESCE((raw_app_meta_data->>'is_system_admin')::boolean, false)"#,
    )
    .fetch_one(&state.pg_pool)
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?;
    if admin_count <= 1 {
      return Err(actix_web::error::ErrorConflict(
        "The last system administrator cannot be revoked",
      ));
    }
  }

  let updated = sqlx::query(
    r#"UPDATE auth.users
          SET raw_app_meta_data = COALESCE(raw_app_meta_data, '{}'::jsonb)
              || jsonb_build_object('is_system_admin', $2::boolean)
        WHERE id = $1"#,
  )
  .bind(target_uuid)
  .bind(payload.enabled)
  .execute(&state.pg_pool)
  .await
  .map_err(actix_web::error::ErrorInternalServerError)?;

  if updated.rows_affected() == 0 {
    return Err(actix_web::error::ErrorNotFound("User not found"));
  }

  tracing::info!(
    actor_uuid = %actor_uuid,
    target_uuid = %target_uuid,
    enabled = payload.enabled,
    "system administrator role changed"
  );
  Ok(AppResponse::Ok().into())
}

async fn list_system_config(
  auth: Authorization,
  state: web::Data<AppState>,
) -> Result<JsonAppResponse<Vec<SystemConfig>>, actix_web::Error> {
  require_system_admin(&state, &auth).await?;
  let configs = sqlx::query_as::<_, SystemConfig>(
    "SELECT key, value FROM af_system_config ORDER BY key",
  )
  .fetch_all(&state.pg_pool)
  .await
  .map_err(actix_web::error::ErrorInternalServerError)?;
  Ok(AppResponse::Ok().with_data(configs).into())
}

async fn upsert_system_config(
  auth: Authorization,
  state: web::Data<AppState>,
  key: web::Path<String>,
  payload: web::Json<UpsertSystemConfig>,
) -> Result<JsonAppResponse<()>, actix_web::Error> {
  let actor_uuid = require_system_admin(&state, &auth).await?;
  let key = key.into_inner();
  if !SUPPORTED_SYSTEM_CONFIG.contains(&key.as_str()) {
    return Err(actix_web::error::ErrorBadRequest("Unsupported config key"));
  }
  if !payload.value.is_boolean() {
    return Err(actix_web::error::ErrorBadRequest(
      "This config value must be a boolean",
    ));
  }
  sqlx::query(
    r#"INSERT INTO af_system_config (key, value, updated_by)
       VALUES ($1, $2, $3)
       ON CONFLICT (key) DO UPDATE
       SET value = EXCLUDED.value, updated_by = EXCLUDED.updated_by, updated_at = NOW()"#,
  )
  .bind(key)
  .bind(&payload.value)
  .bind(actor_uuid)
  .execute(&state.pg_pool)
  .await
  .map_err(actix_web::error::ErrorInternalServerError)?;
  Ok(AppResponse::Ok().into())
}

async fn list_signup_whitelist(
  auth: Authorization,
  state: web::Data<AppState>,
) -> Result<JsonAppResponse<Vec<SignupWhitelistEntry>>, actix_web::Error> {
  require_system_admin(&state, &auth).await?;
  let entries = sqlx::query_as::<_, SignupWhitelistEntry>(
    "SELECT id, kind, value FROM af_signup_whitelist ORDER BY kind, value",
  )
  .fetch_all(&state.pg_pool)
  .await
  .map_err(actix_web::error::ErrorInternalServerError)?;
  Ok(AppResponse::Ok().with_data(entries).into())
}

async fn create_signup_whitelist(
  auth: Authorization,
  state: web::Data<AppState>,
  payload: web::Json<CreateSignupWhitelistEntry>,
) -> Result<JsonAppResponse<SignupWhitelistEntry>, actix_web::Error> {
  let actor_uuid = require_system_admin(&state, &auth).await?;
  let kind = payload.kind.trim().to_lowercase();
  let value = payload.value.trim().trim_start_matches('@').to_lowercase();
  if !matches!(kind.as_str(), "email" | "domain") || value.is_empty() {
    return Err(actix_web::error::ErrorBadRequest(
      "kind must be email or domain and value cannot be empty",
    ));
  }
  if (kind == "email" && (!value.contains('@') || value.starts_with('@')))
    || (kind == "domain" && (value.contains('@') || !value.contains('.')))
  {
    return Err(actix_web::error::ErrorBadRequest("Invalid whitelist value"));
  }
  let entry = sqlx::query_as::<_, SignupWhitelistEntry>(
    r#"INSERT INTO af_signup_whitelist (kind, value, created_by)
       VALUES ($1, $2, $3)
       ON CONFLICT (kind, value) DO UPDATE SET value = EXCLUDED.value
       RETURNING id, kind, value"#,
  )
  .bind(kind)
  .bind(value)
  .bind(actor_uuid)
  .fetch_one(&state.pg_pool)
  .await
  .map_err(actix_web::error::ErrorInternalServerError)?;
  Ok(AppResponse::Ok().with_data(entry).into())
}

async fn delete_signup_whitelist(
  auth: Authorization,
  state: web::Data<AppState>,
  entry_id: web::Path<Uuid>,
) -> Result<JsonAppResponse<()>, actix_web::Error> {
  require_system_admin(&state, &auth).await?;
  let result = sqlx::query("DELETE FROM af_signup_whitelist WHERE id = $1")
    .bind(entry_id.into_inner())
    .execute(&state.pg_pool)
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?;
  if result.rows_affected() == 0 {
    return Err(actix_web::error::ErrorNotFound("Whitelist entry not found"));
  }
  Ok(AppResponse::Ok().into())
}

async fn list_pending_guest_invitations(
  auth: Authorization,
  state: web::Data<AppState>,
) -> Result<JsonAppResponse<Vec<PendingGuestInvitation>>, actix_web::Error> {
  require_system_admin(&state, &auth).await?;
  let invitations = sqlx::query_as::<_, PendingGuestInvitation>(
    r#"SELECT i.id, i.workspace_id, i.invitee_email, i.created_at
         FROM af_workspace_invitation i
        WHERE i.role_id = 3 AND i.status = 0 AND i.admin_approval_status = 0
        ORDER BY i.created_at ASC"#,
  )
  .fetch_all(&state.pg_pool)
  .await
  .map_err(actix_web::error::ErrorInternalServerError)?;
  Ok(AppResponse::Ok().with_data(invitations).into())
}

async fn approve_guest_invitation(
  auth: Authorization,
  state: web::Data<AppState>,
  invite_id: web::Path<Uuid>,
) -> Result<JsonAppResponse<()>, actix_web::Error> {
  set_guest_invitation_approval(auth, state, invite_id.into_inner(), 1).await
}

async fn reject_guest_invitation(
  auth: Authorization,
  state: web::Data<AppState>,
  invite_id: web::Path<Uuid>,
) -> Result<JsonAppResponse<()>, actix_web::Error> {
  set_guest_invitation_approval(auth, state, invite_id.into_inner(), 2).await
}

async fn set_guest_invitation_approval(
  auth: Authorization,
  state: web::Data<AppState>,
  invite_id: Uuid,
  approval_status: i16,
) -> Result<JsonAppResponse<()>, actix_web::Error> {
  let actor_uuid = require_system_admin(&state, &auth).await?;
  let result = sqlx::query(
    r#"UPDATE af_workspace_invitation
          SET admin_approval_status = $2, updated_at = NOW()
        WHERE id = $1 AND role_id = 3 AND status = 0 AND admin_approval_status = 0"#,
  )
  .bind(invite_id)
  .bind(approval_status)
  .execute(&state.pg_pool)
  .await
  .map_err(actix_web::error::ErrorInternalServerError)?;
  if result.rows_affected() == 0 {
    return Err(actix_web::error::ErrorNotFound(
      "Pending guest invitation not found",
    ));
  }
  tracing::info!(
    actor_uuid = %actor_uuid,
    invite_id = %invite_id,
    approval_status,
    "guest invitation approval changed"
  );
  Ok(AppResponse::Ok().into())
}
