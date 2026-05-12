use anyhow::{Context, bail};
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::time::{SystemTime, UNIX_EPOCH};

const BASE_URL: &str = "https://api.appstoreconnect.apple.com/v1";
const TOKEN_LIFETIME_SECONDS: u64 = 15 * 60;

#[derive(Clone)]
pub struct AppStoreConnectClient {
    http: reqwest::Client,
    token_encoder: ConnectTokenEncoder,
}

impl AppStoreConnectClient {
    pub fn from_env() -> anyhow::Result<Self> {
        let key_id = required_env("ASC_KEY_ID")?;
        let issuer_id = required_env("ASC_ISSUER_ID")?;
        let private_key_path = required_env("ASC_PRIVATE_KEY_PATH")?;
        let private_key = std::fs::read(&private_key_path)
            .with_context(|| format!("reading {private_key_path}"))?;
        let encoding_key = EncodingKey::from_ec_pem(&private_key).with_context(|| {
            format!("parsing App Store Connect private key at {private_key_path}")
        })?;

        Ok(Self {
            http: reqwest::Client::new(),
            token_encoder: ConnectTokenEncoder {
                key_id,
                issuer_id,
                encoding_key,
            },
        })
    }

    pub async fn overview_for_bundle_id(
        &self,
        bundle_id: &str,
    ) -> anyhow::Result<Option<AppStoreConnectOverview>> {
        let Some(app) = self.app_for_bundle_id(bundle_id).await? else {
            return Ok(None);
        };

        let product = self.ci_product_for_app(&app.id).await?;
        let workflows = self.workflows_for_product(&product.id).await?;
        let build_runs = self.build_runs_for_product(&product.id, 20).await?;

        Ok(Some(AppStoreConnectOverview {
            app,
            product,
            workflows,
            build_runs,
        }))
    }

    pub async fn list_apps(&self) -> anyhow::Result<Vec<AscApp>> {
        let mut apps = Vec::new();
        let mut response: JsonApiList<AscAppAttributes> =
            self.get("/apps", &[("limit", "200")]).await?;

        loop {
            let next = response.links.as_ref().and_then(|links| links.next.clone());
            apps.extend(response.data.into_iter().map(AscApp::from));

            let Some(next) = next else {
                break;
            };
            if !next.starts_with(BASE_URL) {
                bail!("unexpected App Store Connect pagination URL: {next}");
            }

            response = self.get_url(&next).await?;
        }

        Ok(apps)
    }

    pub async fn app_for_bundle_id(&self, bundle_id: &str) -> anyhow::Result<Option<AscApp>> {
        let response: JsonApiList<AscAppAttributes> = self
            .get("/apps", &[("filter[bundleId]", bundle_id), ("limit", "1")])
            .await?;

        Ok(response.data.into_iter().next().map(AscApp::from))
    }

    pub async fn ci_product_for_app(&self, app_id: &str) -> anyhow::Result<CiProduct> {
        let response: JsonApiSingle<CiProductAttributes> =
            self.get(&format!("/apps/{app_id}/ciProduct"), &[]).await?;

        Ok(response.data.into())
    }

    pub async fn workflows_for_product(&self, product_id: &str) -> anyhow::Result<Vec<CiWorkflow>> {
        let response: JsonApiList<CiWorkflowAttributes> = self
            .get(&format!("/ciProducts/{product_id}/workflows"), &[])
            .await?;

        Ok(response.data.into_iter().map(CiWorkflow::from).collect())
    }

    pub async fn build_runs_for_product(
        &self,
        product_id: &str,
        limit: u16,
    ) -> anyhow::Result<Vec<CiBuildRun>> {
        let limit = limit.to_string();
        let response: JsonApiList<CiBuildRunAttributes> = self
            .get(
                &format!("/ciProducts/{product_id}/buildRuns"),
                &[("sort", "-number"), ("limit", limit.as_str())],
            )
            .await?;

        Ok(response.data.into_iter().map(CiBuildRun::from).collect())
    }

    pub async fn start_build(&self, workflow_id: &str, clean: bool) -> anyhow::Result<CiBuildRun> {
        let attributes = if clean {
            json!({ "clean": true })
        } else {
            json!({})
        };
        let body = json!({
            "data": {
                "type": "ciBuildRuns",
                "attributes": attributes,
                "relationships": {
                    "workflow": {
                        "data": {
                            "type": "ciWorkflows",
                            "id": workflow_id
                        }
                    }
                }
            }
        });

        let response: JsonApiSingle<CiBuildRunAttributes> =
            self.post("/ciBuildRuns", &body).await?;
        Ok(response.data.into())
    }

    async fn get<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> anyhow::Result<T> {
        let url = format!("{BASE_URL}{path}");
        let mut request = self.http.get(&url).bearer_auth(self.token_encoder.token()?);
        if !query.is_empty() {
            request = request.query(query);
        }

        let response = request
            .send()
            .await
            .with_context(|| format!("sending App Store Connect GET {path}"))?;
        decode_response("GET", path, response).await
    }

    async fn get_url<T: DeserializeOwned>(&self, url: &str) -> anyhow::Result<T> {
        let response = self
            .http
            .get(url)
            .bearer_auth(self.token_encoder.token()?)
            .send()
            .await
            .with_context(|| format!("sending App Store Connect GET {url}"))?;
        decode_response("GET", url, response).await
    }

    async fn post<T: DeserializeOwned>(&self, path: &str, body: &Value) -> anyhow::Result<T> {
        let url = format!("{BASE_URL}{path}");
        let response = self
            .http
            .post(&url)
            .bearer_auth(self.token_encoder.token()?)
            .json(body)
            .send()
            .await
            .with_context(|| format!("sending App Store Connect POST {path}"))?;
        decode_response("POST", path, response).await
    }
}

async fn decode_response<T: DeserializeOwned>(
    method: &str,
    path: &str,
    response: reqwest::Response,
) -> anyhow::Result<T> {
    let status = response.status();
    log_rate_limit(method, path, response.headers());

    let bytes = response
        .bytes()
        .await
        .with_context(|| format!("reading App Store Connect {method} {path} response"))?;

    if !status.is_success() {
        let body = String::from_utf8_lossy(&bytes);
        bail!("App Store Connect {method} {path} failed with {status}: {body}");
    }

    serde_json::from_slice(&bytes)
        .with_context(|| format!("decoding App Store Connect {method} {path} response"))
}

fn log_rate_limit(method: &str, path: &str, headers: &reqwest::header::HeaderMap) {
    let Some(value) = headers.get("X-Rate-Limit") else {
        return;
    };

    let Ok(value) = value.to_str() else {
        tracing::debug!(
            method,
            path,
            "App Store Connect X-Rate-Limit header was invalid"
        );
        return;
    };

    let mut user_hour_limit = None;
    let mut user_hour_remaining = None;
    for part in value.split(';') {
        let Some((key, raw_value)) = part.split_once(':') else {
            continue;
        };
        let parsed = raw_value.parse::<u64>().ok();
        match key {
            "user-hour-lim" => user_hour_limit = parsed,
            "user-hour-rem" => user_hour_remaining = parsed,
            _ => {}
        }
    }

    tracing::debug!(
        method,
        path,
        rate_limit = value,
        user_hour_limit,
        user_hour_remaining,
        "App Store Connect rate limit"
    );
}

fn required_env(name: &str) -> anyhow::Result<String> {
    std::env::var(name).with_context(|| format!("missing {name}"))
}

#[derive(Clone)]
struct ConnectTokenEncoder {
    key_id: String,
    issuer_id: String,
    encoding_key: EncodingKey,
}

impl ConnectTokenEncoder {
    fn token(&self) -> anyhow::Result<String> {
        let mut header = Header::new(Algorithm::ES256);
        header.kid = Some(self.key_id.clone());

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("calculating unix time")?
            .as_secs();

        let claims = ConnectTokenClaims {
            iss: self.issuer_id.clone(),
            iat: now,
            exp: now + TOKEN_LIFETIME_SECONDS,
            aud: "appstoreconnect-v1",
        };

        jsonwebtoken::encode(&header, &claims, &self.encoding_key)
            .context("signing App Store Connect JWT")
    }
}

#[derive(Serialize)]
struct ConnectTokenClaims {
    iss: String,
    iat: u64,
    exp: u64,
    aud: &'static str,
}

#[derive(Debug, Clone)]
#[allow(unused)]
pub struct AppStoreConnectOverview {
    pub app: AscApp,
    pub product: CiProduct,
    pub workflows: Vec<CiWorkflow>,
    pub build_runs: Vec<CiBuildRun>,
}

#[derive(Debug, Clone)]
pub struct AscApp {
    pub id: String,
    pub name: Option<String>,
    pub bundle_id: Option<String>,
}

#[derive(Debug, Clone)]
#[allow(unused)]
pub struct CiProduct {
    pub id: String,
    pub name: Option<String>,
    pub product_type: Option<String>,
}

#[derive(Debug, Clone)]
#[allow(unused)]
pub struct CiWorkflow {
    pub id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_enabled: Option<bool>,
    pub is_locked_for_editing: Option<bool>,
}

#[allow(unused)]
impl CiWorkflow {
    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.id)
    }
}

#[derive(Debug, Clone)]
pub struct CiBuildRun {
    pub id: String,
    pub number: Option<u64>,
    pub created_date: Option<String>,
    pub started_date: Option<String>,
    pub finished_date: Option<String>,
    pub execution_progress: Option<String>,
    pub completion_status: Option<String>,
    pub start_reason: Option<String>,
    pub cancel_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct JsonApiList<T> {
    data: Vec<JsonApiResource<T>>,
    links: Option<JsonApiLinks>,
}

#[derive(Debug, Deserialize)]
struct JsonApiLinks {
    next: Option<String>,
}

#[derive(Debug, Deserialize)]
struct JsonApiSingle<T> {
    data: JsonApiResource<T>,
}

#[derive(Debug, Deserialize)]
struct JsonApiResource<T> {
    id: String,
    attributes: Option<T>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AscAppAttributes {
    name: Option<String>,
    bundle_id: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CiProductAttributes {
    name: Option<String>,
    product_type: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CiWorkflowAttributes {
    name: Option<String>,
    description: Option<String>,
    is_enabled: Option<bool>,
    is_locked_for_editing: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CiBuildRunAttributes {
    number: Option<u64>,
    created_date: Option<String>,
    started_date: Option<String>,
    finished_date: Option<String>,
    execution_progress: Option<String>,
    completion_status: Option<String>,
    start_reason: Option<String>,
    cancel_reason: Option<String>,
}

impl From<JsonApiResource<AscAppAttributes>> for AscApp {
    fn from(resource: JsonApiResource<AscAppAttributes>) -> Self {
        let attributes = resource.attributes.unwrap_or_default();
        Self {
            id: resource.id,
            name: attributes.name,
            bundle_id: attributes.bundle_id,
        }
    }
}

impl From<JsonApiResource<CiProductAttributes>> for CiProduct {
    fn from(resource: JsonApiResource<CiProductAttributes>) -> Self {
        let attributes = resource.attributes.unwrap_or_default();
        Self {
            id: resource.id,
            name: attributes.name,
            product_type: attributes.product_type,
        }
    }
}

impl From<JsonApiResource<CiWorkflowAttributes>> for CiWorkflow {
    fn from(resource: JsonApiResource<CiWorkflowAttributes>) -> Self {
        let attributes = resource.attributes.unwrap_or_default();
        Self {
            id: resource.id,
            name: attributes.name,
            description: attributes.description,
            is_enabled: attributes.is_enabled,
            is_locked_for_editing: attributes.is_locked_for_editing,
        }
    }
}

impl From<JsonApiResource<CiBuildRunAttributes>> for CiBuildRun {
    fn from(resource: JsonApiResource<CiBuildRunAttributes>) -> Self {
        let attributes = resource.attributes.unwrap_or_default();
        Self {
            id: resource.id,
            number: attributes.number,
            created_date: attributes.created_date,
            started_date: attributes.started_date,
            finished_date: attributes.finished_date,
            execution_progress: attributes.execution_progress,
            completion_status: attributes.completion_status,
            start_reason: attributes.start_reason,
            cancel_reason: attributes.cancel_reason,
        }
    }
}
