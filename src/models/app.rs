use anyhow::Context;
use seekwel::{HasMany, PersistedModel, model};

use crate::{
    library::app_store_connect::AppStoreConnectClient,
    models::{
        build::{Build, BuildColumns, Timestamp},
        test_flight_build::{TestFlightBuild, TestFlightBuildColumns},
        workflow::{Workflow, WorkflowColumns},
    },
};

#[model]
pub struct App {
    pub id: u64,
    pub asc_id: String,
    pub name: String,
    pub bundle_identifier: String,
    pub built_at: Option<i64>,
    pub sync_error: Option<String>,
    pub auto_build_requested_at: Option<Timestamp>,
    pub auto_build_error: Option<String>,
    #[key = app_id]
    pub builds: HasMany<Build>,
    #[key = app_id]
    pub test_flight_builds: HasMany<TestFlightBuild>,
    #[key = app_id]
    pub workflows: HasMany<Workflow>,
}

impl App {
    pub async fn refresh(mut self, client: &AppStoreConnectClient) -> anyhow::Result<()> {
        tracing::debug!("refreshing {}", self.name);
        let mut app_changed = self.sync_error.is_some();
        let Some(asc_app) = client.app_for_bundle_id(&self.bundle_identifier).await? else {
            return Err(anyhow::anyhow!(
                "did not find ASC app for bundle ID `{}`",
                self.bundle_identifier
            ));
        };

        let product = client.ci_product_for_app(&asc_app.id).await?;
        let workflows = client.workflows_for_product(&product.id).await?;
        for asc_workflow in workflows {
            tracing::info!("importing Xcode Cloud workflow {}", asc_workflow.id);
            Workflow::builder()
                .app(self.clone())
                .asc_id(asc_workflow.id)
                .name(asc_workflow.name)
                .description(asc_workflow.description)
                .is_enabled(asc_workflow.is_enabled)
                .is_locked_for_editing(asc_workflow.is_locked_for_editing)
                .create_or_update_by([WorkflowColumns::AscId])?;
        }

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
                let is_newer = self
                    .built_at
                    .map(|built_at| built_at < asc_built_at)
                    .unwrap_or(true);
                if is_newer {
                    self.built_at = Some(asc_built_at);
                    app_changed = true;
                }
            }
        }

        let test_flight_builds = client.test_flight_builds_for_app(&asc_app.id, 20).await?;
        for asc_build in test_flight_builds {
            tracing::info!("importing TestFlight build {}", asc_build.id);
            TestFlightBuild::builder()
                .app(self.clone())
                .asc_id(asc_build.id)
                .version(asc_build.version)
                .uploaded_date(
                    asc_build
                        .uploaded_date
                        .as_ref()
                        .map(|v| v.parse::<Timestamp>())
                        .transpose()?,
                )
                .expiration_date(
                    asc_build
                        .expiration_date
                        .as_ref()
                        .map(|v| v.parse::<Timestamp>())
                        .transpose()?,
                )
                .expired(asc_build.expired)
                .processing_state(asc_build.processing_state)
                .create_or_update_by([TestFlightBuildColumns::AscId])?;
        }

        if app_changed {
            self.sync_error = None;
            self.save().context("saving refreshed app")?;
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

    pub fn current_test_flight_build(&self) -> anyhow::Result<Option<TestFlightBuild>> {
        let mut builds = self.test_flight_builds()?;
        builds.sort_by_key(|build| (build.uploaded_date, build.expiration_date));
        builds.reverse();

        if let Some(build) = builds
            .iter()
            .find(|build| build.is_valid() && !build.is_expired())
        {
            return Ok(Some(build.clone()));
        }

        if let Some(build) = builds.iter().find(|build| build.is_valid()) {
            return Ok(Some(build.clone()));
        }

        Ok(builds.first().cloned())
    }

    pub fn current_valid_test_flight_build(&self) -> anyhow::Result<Option<TestFlightBuild>> {
        let mut builds = self.test_flight_builds()?;
        builds.sort_by_key(|build| (build.uploaded_date, build.expiration_date));
        builds.reverse();

        Ok(builds.into_iter().find(|build| build.is_valid()))
    }

    pub fn needs_auto_build(&self) -> anyhow::Result<bool> {
        if self.auto_build_cooldown_active() {
            return Ok(false);
        }

        let Some(build) = self.current_valid_test_flight_build()? else {
            return Ok(true);
        };

        Ok(build.is_expired() || build.expires_within_days(7))
    }

    pub fn auto_build_cooldown_active(&self) -> bool {
        self.auto_build_requested_at
            .is_some_and(|requested_at| requested_at.is_within_last_hours(24))
    }

    pub fn workflows_for_build_start(&self) -> anyhow::Result<Vec<Workflow>> {
        let mut workflows = self.workflows()?;
        workflows.sort_by_key(|workflow| workflow.display_name().to_lowercase());
        Ok(workflows)
    }

    pub fn first_startable_workflow(&self) -> anyhow::Result<Option<Workflow>> {
        Ok(self
            .workflows_for_build_start()?
            .into_iter()
            .find(|workflow| workflow.can_start()))
    }
}
