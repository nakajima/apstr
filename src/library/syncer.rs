use std::{fs, io::ErrorKind, path::Path, time::Duration};

use anyhow::{Context, bail};
use seekwel::{Comparison as Q, ModelQueryDsl, PersistedModel, QueryDsl};

use crate::{
    library::app_store_connect::AppStoreConnectClient,
    models::{
        app::{App, AppColumns},
        build::{Build, BuildColumns, Timestamp},
    },
};

const SYNC_INTERVAL: Duration = Duration::from_secs(60);
const LAST_SYNC_PATH: &str = "target/apstr/last_sync_at";

#[derive(Clone)]
pub struct Syncer {
    client: AppStoreConnectClient,
}

impl Syncer {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            client: AppStoreConnectClient::from_env()?,
        })
    }

    pub async fn start(self) -> ! {
        loop {
            if let Some(delay) = self.sync_delay() {
                tracing::debug!(?delay, "skipping sync; last sync stamp is fresh");
                tokio::time::sleep(delay).await;
                continue;
            }

            match self.fetch_apps().await {
                Ok(()) => {
                    if let Err(error) = self.write_stamp() {
                        tracing::warn!(?error, "failed to write sync stamp");
                    }
                }
                Err(error) => {
                    tracing::error!(?error, "error syncing");
                }
            }

            tokio::time::sleep(SYNC_INTERVAL).await;
        }
    }

    pub async fn fetch_apps(&self) -> anyhow::Result<()> {
        let apps = self.client.list_apps().await?;
        let mut processed_apps = 0;
        let mut refresh_errors = 0;
        for asc_app in apps {
            let Some(name) = asc_app.name else {
                continue;
            };

            let Some(bundle_id) = asc_app.bundle_id else {
                continue;
            };

            let asc_id = asc_app.id;
            let mut app = if let Some(app) =
                App::q(AppColumns::BundleIdentifier, Q::Eq(bundle_id.clone()))
                    .first()
                    .with_context(|| format!("loading local app for bundle ID {bundle_id}"))?
            {
                tracing::debug!("refreshing {}", name);
                app
            } else {
                tracing::debug!("creating local record for {}", name);
                App::builder()
                    .name(name.clone())
                    .asc_id(asc_id.clone())
                    .bundle_identifier(&bundle_id)
                    .create()
                    .with_context(|| format!("creating local app for bundle ID {bundle_id}"))?
            };

            let mut app_changed = false;
            if app.name != name {
                app.name = name;
                app_changed = true;
            }
            if app.asc_id != asc_id {
                app.asc_id = asc_id;
                app_changed = true;
            }
            if app_changed {
                app.save().with_context(|| {
                    format!("saving App Store Connect metadata for app {}", app.id)
                })?;
            }

            processed_apps += 1;
            if let Err(error) = Self::refresh_app(app, &self.client).await {
                refresh_errors += 1;
                tracing::error!(?error, "app refresh failed");
            }
        }

        if refresh_errors > 0 {
            bail!("{refresh_errors} app refreshes failed");
        }

        tracing::info!("processed {} apps", processed_apps);

        Ok(())
    }

    async fn refresh_app(app: App, client: &AppStoreConnectClient) -> anyhow::Result<()> {
        let app_id = app.id;
        let app_name = app.name.clone();

        if let Err(error) = app.refresh(client).await {
            let sync_error = format!("{error:#}");
            tracing::error!(app_id, app_name, error = %sync_error, "failed to refresh app");

            let mut app = App::find(app_id)
                .with_context(|| format!("loading app {app_id} to record sync error"))?;
            app.sync_error = Some(sync_error);
            app.save()
                .with_context(|| format!("saving sync error for app {app_id}"))?;
            return Ok(());
        }

        let app = App::find(app_id)
            .with_context(|| format!("loading app {app_id} for automatic build check"))?;
        if let Err(error) = Self::start_auto_build_if_needed(app, client).await {
            let auto_build_error = format!("{error:#}");
            tracing::error!(app_id, app_name, error = %auto_build_error, "automatic build failed");

            let mut app = App::find(app_id)
                .with_context(|| format!("loading app {app_id} to record automatic build error"))?;
            app.auto_build_error = Some(auto_build_error);
            app.save()
                .with_context(|| format!("saving automatic build error for app {app_id}"))?;
        }

        Ok(())
    }

    async fn start_auto_build_if_needed(
        mut app: App,
        client: &AppStoreConnectClient,
    ) -> anyhow::Result<()> {
        let Some(workflow) = app.first_startable_workflow()? else {
            return Ok(());
        };

        if !app.needs_auto_build()? {
            return Ok(());
        }

        let workflow_id = workflow.asc_id.clone();
        let workflow_name = workflow.display_name().to_string();
        tracing::info!(
            app_id = app.id,
            app_name = %app.name,
            workflow_id = %workflow_id,
            workflow_name = %workflow_name,
            "starting automatic clean build"
        );

        let asc_build = client
            .start_build(&workflow_id, true)
            .await
            .with_context(|| format!("starting automatic workflow {workflow_id}"))?;

        app.auto_build_requested_at = Some(Timestamp::now());
        app.auto_build_error = None;
        app.save()
            .with_context(|| format!("saving automatic build request for app {}", app.id))?;

        Build::builder()
            .app(app)
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
            .create_or_update_by([BuildColumns::AscId])
            .context("saving automatically started build")?;

        Ok(())
    }

    fn sync_delay(&self) -> Option<Duration> {
        let metadata = match fs::metadata(LAST_SYNC_PATH) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == ErrorKind::NotFound => return None,
            Err(error) => {
                tracing::warn!(%error, path = LAST_SYNC_PATH, "failed to read sync stamp");
                return None;
            }
        };

        let modified = match metadata.modified() {
            Ok(modified) => modified,
            Err(error) => {
                tracing::warn!(%error, path = LAST_SYNC_PATH, "failed to read sync stamp mtime");
                return None;
            }
        };

        let elapsed = match modified.elapsed() {
            Ok(elapsed) => elapsed,
            Err(error) => {
                tracing::warn!(%error, path = LAST_SYNC_PATH, "sync stamp mtime is in the future");
                return Some(SYNC_INTERVAL);
            }
        };

        if elapsed < SYNC_INTERVAL {
            Some(SYNC_INTERVAL - elapsed)
        } else {
            None
        }
    }

    fn write_stamp(&self) -> anyhow::Result<()> {
        let path = Path::new(LAST_SYNC_PATH);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating sync stamp directory {}", parent.display()))?;
        }

        fs::write(path, format!("{}\n", jiff::Timestamp::now()))
            .with_context(|| format!("writing sync stamp {}", path.display()))?;

        Ok(())
    }
}
