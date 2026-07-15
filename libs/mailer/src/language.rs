/// The language a mailer template should be rendered in.
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

  /// Parses a persisted language code (e.g. from the `af_user.language`
  /// column). Unknown or missing codes fall back to the default (`En`).
  pub fn from_code(code: Option<&str>) -> Self {
    match code {
      Some("fr") => Language::Fr,
      _ => Language::En,
    }
  }
}
