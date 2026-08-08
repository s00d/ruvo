//! Feature flags (Laravel Fortify-style).

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Feature {
    Registration,
    ResetPasswords,
    EmailVerification,
    UpdateProfile,
    UpdatePasswords,
    TwoFactor,
    Roles,
}

impl Feature {
    pub fn all() -> &'static [Feature] {
        &[
            Feature::Registration,
            Feature::ResetPasswords,
            Feature::EmailVerification,
            Feature::UpdateProfile,
            Feature::UpdatePasswords,
            Feature::TwoFactor,
            Feature::Roles,
        ]
    }
}
