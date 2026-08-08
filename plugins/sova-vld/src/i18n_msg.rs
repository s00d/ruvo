//! Map `IssueCode` → i18n keys (`validation.*`).

use crate::error::{issue_code_slug, ValidationError};
use sova_core::Request;
use sova_i18n::{I18nExt, I18nState};
use vld::error::IssueCode;

pub fn localize(mut err: ValidationError, req: &Request) -> ValidationError {
    let Some(state) = req.try_state::<I18nState>() else {
        return err;
    };
    let locale = req.locale();
    for issue in &mut err.0.issues {
        let key = match &issue.code {
            IssueCode::MissingField => "required",
            IssueCode::InvalidString { validation }
                if format!("{validation:?}")
                    .to_ascii_lowercase()
                    .contains("email") =>
            {
                "email"
            }
            IssueCode::TooSmall { .. } => "min",
            IssueCode::TooBig { .. } => "max",
            other => {
                let slug = issue_code_slug(other);
                let translated = state.translate(locale, "validation", &slug);
                if translated != slug {
                    issue.message = translated;
                }
                continue;
            }
        };
        let translated = state.translate(locale, "validation", key);
        if translated != key {
            issue.message = translated;
        }
    }
    err
}
