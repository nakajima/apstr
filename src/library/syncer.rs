use std::time::Duration;

use seekwel::{Comparison as Q, ModelQueryDsl, QueryDsl};
use tokio::task::JoinSet;

use crate::{
    library::app_store_connect::AppStoreConnectClient,
    models::{
        app::{App, AppColumns},
        build::Timestamp,
    },
};

#[derive(Clone)]
pub struct Syncer {
    last_sync_at: Option<Timestamp>,
}

impl Syncer {
    #[allow(unused)]
    pub fn new() -> Self {
        Self { last_sync_at: None }
    }

    #[allow(unused)]
    pub async fn start(&mut self) -> ! {
        loop {
            if let Err(e) = self.fetch_apps().await {
                tracing::error!("Error syncing: {e:?}");
            };
            std::thread::sleep(Duration::from_secs(3));
        }
    }

    pub async fn fetch_apps(&mut self) -> anyhow::Result<()> {
        let client = AppStoreConnectClient::from_env()?;
        let apps = client.list_apps().await?;
        let mut join_set = JoinSet::new();
        for asc_app in apps {
            let Some(name) = asc_app.name else {
                continue;
            };

            let Some(bundle_id) = asc_app.bundle_id else {
                continue;
            };

            let app = if let Some(app) =
                App::q(AppColumns::BundleIdentifier, Q::Eq(bundle_id.clone()))
                    .first()
                    .ok()
                    .flatten()
            {
                tracing::debug!("refreshing {}", name);
                app
            } else {
                tracing::debug!("creating local record for {}", name);
                App::builder()
                    .name(name)
                    .bundle_identifier(&bundle_id)
                    .create()?
            };

            join_set.spawn(app.refresh());
        }

        let len = join_set.len();
        join_set.join_all().await;
        tracing::info!("refreshed {} apps", len);

        self.last_sync_at = Some(Timestamp::now());

        Ok(())
    }
}
