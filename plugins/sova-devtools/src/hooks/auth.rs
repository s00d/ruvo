use crate::collector::{AuthSnap, DevToolsBag};

#[cfg(feature = "session")]
pub fn collect_session_auth(
    bag: &DevToolsBag,
    session: Option<sova_session::Session>,
    #[cfg(feature = "auth")] user: Option<sova_auth::CurrentUser>,
    #[cfg(feature = "passport")] passport: Option<sova_passport::Authenticated>,
) {
    use crate::redact::mask_value;
    if let Some(sess) = session {
        let mut keys = Vec::new();
        for (k, v) in sess.data() {
            keys.push((k.clone(), mask_value(&k, &v)));
        }
        keys.sort_by(|a, b| a.0.cmp(&b.0));
        #[allow(unused_mut)]
        let mut auth = AuthSnap {
            session_id: Some(sess.id()),
            user_id: sess.user_id(),
            email: None,
            roles: Vec::new(),
            session_keys: keys,
        };
        #[cfg(feature = "auth")]
        if let Some(u) = user {
            auth.user_id = Some(u.id.to_string());
            auth.email = Some(u.email.clone());
            auth.roles = u.roles.clone();
        }
        #[cfg(feature = "passport")]
        if auth.user_id.is_none() {
            if let Some(p) = passport {
                auth.user_id = Some(p.id);
            }
        }
        bag.set_auth(auth);
    } else {
        #[cfg(any(feature = "auth", feature = "passport"))]
        fill_auth_without_session(
            bag,
            #[cfg(feature = "auth")]
            user,
            #[cfg(feature = "passport")]
            passport,
        );
    }
}

#[cfg(any(feature = "auth", feature = "passport"))]
pub fn fill_auth_without_session(
    bag: &DevToolsBag,
    #[cfg(feature = "auth")] user: Option<sova_auth::CurrentUser>,
    #[cfg(feature = "passport")] passport: Option<sova_passport::Authenticated>,
) {
    #[allow(unused_mut)]
    let mut auth = AuthSnap::default();
    let mut any = false;
    #[cfg(feature = "auth")]
    if let Some(u) = user {
        auth.user_id = Some(u.id.to_string());
        auth.email = Some(u.email);
        auth.roles = u.roles;
        any = true;
    }
    #[cfg(feature = "passport")]
    if let Some(p) = passport {
        if auth.user_id.is_none() {
            auth.user_id = Some(p.id);
            any = true;
        }
    }
    if any {
        bag.set_auth(auth);
    }
}
