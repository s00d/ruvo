//! ACME order: HTTP-01 issue / renew via instant-acme.

use crate::http01::ChallengeMap;
use crate::storage::AcmeStorage;
use instant_acme::{
    Account, AuthorizationStatus, ChallengeType, Identifier, LetsEncrypt, NewAccount, NewOrder,
    OrderStatus, RetryPolicy,
};
use sova_core::{Error, Result};
use std::time::Duration;

pub struct IssuedCert {
    pub cert_pem: String,
    pub key_pem: String,
}

pub async fn obtain_certificate(
    domains: &[String],
    email: Option<&str>,
    staging: bool,
    storage: &AcmeStorage,
    challenges: &ChallengeMap,
) -> Result<IssuedCert> {
    let directory = if staging {
        LetsEncrypt::Staging.url()
    } else {
        LetsEncrypt::Production.url()
    };

    let account = if let Some(json) = storage.load_account_json() {
        let creds = serde_json::from_str(&json)
            .map_err(|e| Error::Internal(format!("acme account parse: {e}")))?;
        Account::builder()
            .map_err(|e| Error::Internal(format!("acme account builder: {e}")))?
            .from_credentials(creds)
            .await
            .map_err(|e| Error::Internal(format!("acme account restore: {e}")))?
    } else {
        let contact: Vec<String> = email
            .map(|e| {
                if e.starts_with("mailto:") {
                    e.to_string()
                } else {
                    format!("mailto:{e}")
                }
            })
            .into_iter()
            .collect();
        let contact_refs: Vec<&str> = contact.iter().map(|s| s.as_str()).collect();
        let (account, credentials) = Account::builder()
            .map_err(|e| Error::Internal(format!("acme account builder: {e}")))?
            .create(
                &NewAccount {
                    contact: &contact_refs,
                    terms_of_service_agreed: true,
                    only_return_existing: false,
                },
                directory.to_owned(),
                None,
            )
            .await
            .map_err(|e| Error::Internal(format!("acme create account: {e}")))?;
        let json = serde_json::to_string_pretty(&credentials)
            .map_err(|e| Error::Internal(format!("acme credentials serialize: {e}")))?;
        storage.save_account_json(&json)?;
        account
    };

    let identifiers: Vec<Identifier> = domains.iter().map(|d| Identifier::Dns(d.clone())).collect();
    let mut order = account
        .new_order(&NewOrder::new(identifiers.as_slice()))
        .await
        .map_err(|e| Error::Internal(format!("acme new order: {e}")))?;

    let mut authorizations = order.authorizations();
    let mut tokens = Vec::new();
    while let Some(result) = authorizations.next().await {
        let mut authz = result.map_err(|e| Error::Internal(format!("acme authz: {e}")))?;
        match authz.status {
            AuthorizationStatus::Pending => {}
            AuthorizationStatus::Valid => continue,
            other => {
                return Err(Error::Internal(format!("acme authz status: {other:?}")));
            }
        }

        let mut challenge = authz
            .challenge(ChallengeType::Http01)
            .ok_or_else(|| Error::Internal("acme: no HTTP-01 challenge".into()))?;

        let token = challenge.token.clone();
        let key_auth = challenge.key_authorization().as_str().to_string();
        challenges.insert(&token, &key_auth);
        tokens.push(token);

        tokio::time::sleep(Duration::from_millis(50)).await;

        challenge
            .set_ready()
            .await
            .map_err(|e| Error::Internal(format!("acme set_ready: {e}")))?;
    }

    let status = order
        .poll_ready(&RetryPolicy::default())
        .await
        .map_err(|e| Error::Internal(format!("acme poll_ready: {e}")))?;
    if status != OrderStatus::Ready {
        for t in &tokens {
            challenges.remove(t);
        }
        return Err(Error::Internal(format!(
            "acme unexpected order status: {status:?}"
        )));
    }

    let key_pem = order
        .finalize()
        .await
        .map_err(|e| Error::Internal(format!("acme finalize: {e}")))?;
    let cert_pem = order
        .poll_certificate(&RetryPolicy::default())
        .await
        .map_err(|e| Error::Internal(format!("acme poll_certificate: {e}")))?;

    for t in &tokens {
        challenges.remove(t);
    }

    Ok(IssuedCert { cert_pem, key_pem })
}
