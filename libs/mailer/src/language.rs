/// The language a mailer template should be rendered in.
///
/// There is currently no persisted per-user/workspace language preference,
/// so every call site explicitly passes a `Language` and defaults to `En`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Language {
  #[default]
  En,
  Fr,
}

impl Language {
  /// Suffix used to derive the localized handlebars template name
  /// (e.g. `workspace_invite` -> `workspace_invite_fr`).
  pub fn template_suffix(&self) -> &'static str {
    match self {
      Language::En => "en",
      Language::Fr => "fr",
    }
  }
}
