use crate::import_worker::report::{ImportNotifier, ImportProgress};
use crate::mailer::AFWorkerMailer;
use axum::async_trait;
use mailer::Language;
use sqlx::PgPool;
use tracing::{error, trace};

pub struct EmailNotifier {
  mailer: AFWorkerMailer,
  pg_pool: PgPool,
}
impl EmailNotifier {
  pub fn new(mailer: AFWorkerMailer, pg_pool: PgPool) -> Self {
    Self { mailer, pg_pool }
  }
}

#[async_trait]
impl ImportNotifier for EmailNotifier {
  async fn notify_progress(&self, progress: ImportProgress) {
    match progress {
      ImportProgress::Started { workspace_id: _ } => {},
      ImportProgress::Finished(result) => {
        trace!(
          "[Import]: sending import notion report email to {}, params: {:?}",
          result.user_email,
          result,
        );

        let recipient_language = Language::from_code(
          database::user::select_language_from_email(&self.pg_pool, &result.user_email)
            .await
            .unwrap_or_default()
            .as_deref(),
        );

        if let Err(err) = self
          .mailer
          .send_import_report(
            result.user_name,
            &result.user_email,
            result.is_success,
            result.value,
            recipient_language,
          )
          .await
        {
          error!("Failed to send import notion report email: {}", err);
        }
      },
    }
  }
}
