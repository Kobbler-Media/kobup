use std::time::Instant;

use jsonwebtoken::{Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use crate::http;

#[derive(Deserialize)]
struct ServiceAccount {
    project_id: String,
    client_email: String,
    private_key: String,
}

#[derive(Serialize)]
struct JwtClaims {
    iss: String,
    scope: String,
    aud: String,
    iat: i64,
    exp: i64,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

fn mint_assertion_jwt(
    account: &ServiceAccount,
    now: i64,
) -> Result<String, jsonwebtoken::errors::Error> {
    let claims = JwtClaims {
        iss: account.client_email.clone(),
        scope: "https://www.googleapis.com/auth/firebase.messaging".to_string(),
        aud: "https://oauth2.googleapis.com/token".to_string(),
        iat: now,
        exp: now + 3600,
    };
    let key = EncodingKey::from_rsa_pem(account.private_key.as_bytes())?;
    jsonwebtoken::encode(&Header::new(Algorithm::RS256), &claims, &key)
}

pub struct FcmClient {
    service_account: ServiceAccount,
    http_client: reqwest::Client,
    access_token: RwLock<Option<(String, Instant)>>,
}

impl FcmClient {
    pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = std::fs::read_to_string(path)?;
        let service_account: ServiceAccount = serde_json::from_str(&contents)?;
        Ok(Self {
            service_account,
            http_client: http::client(),
            access_token: RwLock::new(None),
        })
    }

    async fn get_access_token(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // Check cached token
        {
            let cached = self.access_token.read().await;
            if let Some((ref token, ref issued_at)) = *cached {
                // Refresh 5 minutes before expiry (tokens last 1 hour)
                if issued_at.elapsed().as_secs() < 3300 {
                    return Ok(token.clone());
                }
            }
        }

        let jwt = mint_assertion_jwt(&self.service_account, chrono::Utc::now().timestamp())?;

        // Exchange for access token
        let form_body = format!(
            "grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Ajwt-bearer&assertion={jwt}"
        );
        let resp = self
            .http_client
            .post("https://oauth2.googleapis.com/token")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(form_body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("token exchange failed ({status}): {body}").into());
        }

        let token_resp: TokenResponse = resp.json().await?;
        let token = token_resp.access_token;

        // Cache it
        {
            let mut cached = self.access_token.write().await;
            *cached = Some((token.clone(), Instant::now()));
        }

        Ok(token)
    }

    pub async fn send_refresh(
        &self,
        tokens: &[String],
        project_slug: &str,
        channel: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let access_token = self.get_access_token().await?;
        let url = format!(
            "https://fcm.googleapis.com/v1/projects/{}/messages:send",
            self.service_account.project_id
        );

        let mut success_count = 0u32;
        let mut fail_count = 0u32;

        for device_token in tokens {
            let payload = serde_json::json!({
                "message": {
                    "token": device_token,
                    "data": {
                        "type": "refresh",
                        "projectSlug": project_slug,
                        "channel": channel,
                    }
                }
            });

            match self
                .http_client
                .post(&url)
                .bearer_auth(&access_token)
                .json(&payload)
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => {
                    success_count += 1;
                }
                Ok(resp) => {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    warn!(
                        device_token,
                        status = %status,
                        body,
                        "FCM send failed for device"
                    );
                    fail_count += 1;
                }
                Err(e) => {
                    warn!(device_token, error = %e, "FCM send error for device");
                    fail_count += 1;
                }
            }
        }

        info!(
            success_count,
            fail_count, project_slug, channel, "FCM refresh batch complete"
        );

        if fail_count > 0 && success_count == 0 {
            error!("all FCM sends failed");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{DecodingKey, Validation};

    const TEST_ISSUER: &str = "fcm@kobup-test.iam.gserviceaccount.com";
    const TEST_AUDIENCE: &str = "https://oauth2.googleapis.com/token";

    #[test]
    fn rs256_assertion_jwt_signs_and_verifies() {
        let account = ServiceAccount {
            project_id: "kobup-test".to_string(),
            client_email: TEST_ISSUER.to_string(),
            private_key: include_str!("../tests/fixtures/fcm_test_key.pem").to_string(),
        };
        let now = 1_700_000_000;

        let jwt = mint_assertion_jwt(&account, now).expect("RS256 signing failed");

        let key = DecodingKey::from_rsa_pem(
            include_str!("../tests/fixtures/fcm_test_key.pub.pem").as_bytes(),
        )
        .expect("public key parse failed");
        let mut validation = Validation::new(Algorithm::RS256);
        validation.validate_exp = false;
        validation.set_issuer(&[TEST_ISSUER]);
        validation.set_audience(&[TEST_AUDIENCE]);

        let decoded = jsonwebtoken::decode::<serde_json::Value>(&jwt, &key, &validation)
            .expect("RS256 verification failed");

        assert_eq!(decoded.header.alg, Algorithm::RS256);
        assert_eq!(decoded.claims["iss"], TEST_ISSUER);
        assert_eq!(decoded.claims["aud"], TEST_AUDIENCE);
        assert_eq!(decoded.claims["iat"], now);
        assert_eq!(decoded.claims["exp"], now + 3600);
        assert_eq!(
            decoded.claims["scope"],
            "https://www.googleapis.com/auth/firebase.messaging"
        );
    }
}
