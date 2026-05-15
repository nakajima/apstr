use seekwel::{HasMany, model};

use crate::{
    library::app_store_connect::AppStoreConnectClient,
    models::build::{Build, BuildColumns, Timestamp},
};

#[model]
pub struct App {
    pub id: u64,
    pub asc_id: String,
    pub name: String,
    pub bundle_identifier: String,
    pub built_at: Option<i64>,
    pub builds: HasMany<Build, { BuildColumns::APP_ID }>,
}

impl App {
    pub async fn refresh(mut self) -> anyhow::Result<()> {
        tracing::debug!("refreshing {}", self.name);
        let client = AppStoreConnectClient::from_env()?;
        let Some(asc_app) = client.app_for_bundle_id(&self.bundle_identifier).await? else {
            return Err(anyhow::anyhow!(
                "did not find ASC app for bundle ID `{}`",
                self.bundle_identifier
            ));
        };

        let product = client.ci_product_for_app(&asc_app.id).await?;
        let builds = client.build_runs_for_product(&product.id, 10).await?;
        for asc_build in builds {
            tracing::info!("importing ASC build {}", asc_build.id);
            Build::builder()
                .app(self.clone())
                .asc_id(asc_build.id)
                .number(asc_build.number)
                .created_date(
                    asc_build
                        .created_date
                        .as_ref()
                        .map(|v| v.parse::<Timestamp>())
                        .transpose()?,
                )
                .started_date(
                    asc_build
                        .started_date
                        .as_ref()
                        .map(|v| v.parse::<Timestamp>())
                        .transpose()?,
                )
                .finished_date(
                    asc_build
                        .finished_date
                        .as_ref()
                        .map(|v| v.parse::<Timestamp>())
                        .transpose()?,
                )
                .execution_progress(asc_build.execution_progress)
                .completion_status(asc_build.completion_status)
                .start_reason(asc_build.start_reason)
                .cancel_reason(asc_build.cancel_reason)
                .create_or_update_by([BuildColumns::AscId])?;

            // TODO: This is messy
            if let Some(asc_built_at) = asc_build.started_date.map(|d| d.parse::<Timestamp>()) {
                let asc_built_at = asc_built_at?.into();
                if let Some(built_at) = &self.built_at
                    && *built_at < asc_built_at
                {
                    self.built_at = Some(asc_built_at);
                } else if self.built_at.is_none() {
                    self.built_at = Some(asc_built_at);
                }
            }
        }

        Ok(())
    }

    pub fn latest_build(&self) -> anyhow::Result<Option<Build>> {
        let mut builds = self.builds()?;
        // TODO: this should be do-able in sql
        builds.sort_by_key(|a| a.number.unwrap_or(0));
        builds.reverse();
        Ok(builds.first().cloned())
    }
}
