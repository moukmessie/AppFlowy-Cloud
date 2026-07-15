use crate::import_worker::report::{ImportNotifier, ImportProgress};
use crate::mailer::AFWorkerMailer;
use axum::async_trait;
use mailer::Language;
use tracing::{error, trace};

pub struct EmailNotifier(AFWorkerMailer);
impl EmailNotifier {
  pub fn new(mailer: AFWorkerMailer) -> Self {
    Self(mailer)
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

        if let Err(err) = self
          .0
          .send_import_report(
            result.user_name,
            &result.user_email,
            result.is_success,
            result.value,
            Language::En,
          )
          .await
        {
          error!("Failed to send import notion report email: {}", err);
        }
      },
    }
  }
}
