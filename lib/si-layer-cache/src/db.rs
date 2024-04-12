use std::{future::IntoFuture, io, path::Path, sync::Arc};

use serde::{de::DeserializeOwned, Serialize};
use si_data_nats::NatsClient;
use si_data_pg::PgPool;
use telemetry::prelude::*;
use tokio::sync::mpsc;
use tokio_util::{sync::CancellationToken, task::TaskTracker};
use ulid::Ulid;

use crate::db::encrypted_secret::EncryptedSecretDb;
use crate::{
    activity_client::ActivityClient,
    error::LayerDbResult,
    layer_cache::LayerCache,
    persister::{PersisterClient, PersisterTask},
};

use self::{
    cache_updates::CacheUpdatesTask, cas::CasDb, func_execution::FuncExecutionDb,
    workspace_snapshot::WorkspaceSnapshotDb,
};

mod cache_updates;
pub mod cas;
pub mod encrypted_secret;
pub mod func_execution;
pub mod serialize;
pub mod workspace_snapshot;

#[derive(Debug, Clone)]
pub struct LayerDb<CasValue, EncryptedSecretValue, FuncExecutionValue, WorkspaceSnapshotValue>
where
    CasValue: Serialize + DeserializeOwned + Clone + Send + Sync + 'static,
    EncryptedSecretValue: Serialize + DeserializeOwned + Clone + Send + Sync + 'static,
    FuncExecutionValue: Serialize + DeserializeOwned + Clone + Send + Sync + 'static,
    WorkspaceSnapshotValue: Serialize + DeserializeOwned + Clone + Send + Sync + 'static,
{
    cas: CasDb<CasValue>,
    encrypted_secret: EncryptedSecretDb<EncryptedSecretValue>,
    func_execution: FuncExecutionDb<FuncExecutionValue>,
    workspace_snapshot: WorkspaceSnapshotDb<WorkspaceSnapshotValue>,
    pg_pool: PgPool,
    nats_client: NatsClient,
    persister_client: PersisterClient,
    activity: ActivityClient,
    instance_id: Ulid,
}

impl<CasValue, EncryptedSecretValue, FuncExecutionValue, WorkspaceSnapshotValue>
    LayerDb<CasValue, EncryptedSecretValue, FuncExecutionValue, WorkspaceSnapshotValue>
where
    CasValue: Serialize + DeserializeOwned + Clone + Send + Sync + 'static,
    EncryptedSecretValue: Serialize + DeserializeOwned + Clone + Send + Sync + 'static,
    FuncExecutionValue: Serialize + DeserializeOwned + Clone + Send + Sync + 'static,
    WorkspaceSnapshotValue: Serialize + DeserializeOwned + Clone + Send + Sync + 'static,
{
    pub async fn initialize(
        disk_path: impl AsRef<Path>,
        pg_pool: PgPool,
        nats_client: NatsClient,
        token: CancellationToken,
    ) -> LayerDbResult<(Self, LayerDbGracefulShutdown)> {
        let instance_id = Ulid::new();

        let tracker = TaskTracker::new();

        let disk_path = disk_path.as_ref();

        let (tx, rx) = mpsc::unbounded_channel();
        let persister_client = PersisterClient::new(tx);

        let cas_cache: LayerCache<Arc<CasValue>> =
            LayerCache::new(cas::CACHE_NAME, disk_path, pg_pool.clone())?;

        let encrypted_secret_cache: LayerCache<Arc<EncryptedSecretValue>> =
            LayerCache::new(encrypted_secret::CACHE_NAME, disk_path, pg_pool.clone())?;

        let func_execution_cache: LayerCache<Arc<FuncExecutionValue>> =
            LayerCache::new(func_execution::CACHE_NAME, disk_path, pg_pool.clone())?;

        let snapshot_cache: LayerCache<Arc<WorkspaceSnapshotValue>> =
            LayerCache::new(workspace_snapshot::CACHE_NAME, disk_path, pg_pool.clone())?;

        let cache_updates_task = CacheUpdatesTask::create(
            instance_id,
            &nats_client,
            cas_cache.clone(),
            encrypted_secret_cache.clone(),
            snapshot_cache.clone(),
            token.clone(),
        )
        .await?;
        tracker.spawn(cache_updates_task.run());

        let persister_task = PersisterTask::create(
            rx,
            disk_path.to_path_buf(),
            pg_pool.clone(),
            &nats_client,
            instance_id,
            token.clone(),
        )
        .await?;
        tracker.spawn(persister_task.run());

        let cas = CasDb::new(cas_cache, persister_client.clone());
        let encrypted_secret =
            EncryptedSecretDb::new(encrypted_secret_cache, persister_client.clone());
        let func_execution = FuncExecutionDb::new(func_execution_cache);
        let workspace_snapshot = WorkspaceSnapshotDb::new(snapshot_cache, persister_client.clone());

        let activity = ActivityClient::new(instance_id, nats_client.clone(), token.clone());
        let graceful_shutdown = LayerDbGracefulShutdown { tracker, token };

        let layerdb = LayerDb {
            activity,
            cas,
            encrypted_secret,
            func_execution,
            workspace_snapshot,
            pg_pool,
            persister_client,
            nats_client,
            instance_id,
        };

        Ok((layerdb, graceful_shutdown))
    }

    pub fn pg_pool(&self) -> &PgPool {
        &self.pg_pool
    }

    pub fn nats_client(&self) -> &NatsClient {
        &self.nats_client
    }

    pub fn persister_client(&self) -> &PersisterClient {
        &self.persister_client
    }

    pub fn cas(&self) -> &CasDb<CasValue> {
        &self.cas
    }

    pub fn encrypted_secret(&self) -> &EncryptedSecretDb<EncryptedSecretValue> {
        &self.encrypted_secret
    }

    pub fn func_execution(&self) -> &FuncExecutionDb<FuncExecutionValue> {
        &self.func_execution
    }

    pub fn workspace_snapshot(&self) -> &WorkspaceSnapshotDb<WorkspaceSnapshotValue> {
        &self.workspace_snapshot
    }

    pub fn instance_id(&self) -> Ulid {
        self.instance_id
    }

    pub fn activity(&self) -> &ActivityClient {
        &self.activity
    }

    /// Run all migrations
    pub async fn pg_migrate(&self) -> LayerDbResult<()> {
        // This will do all migrations, not just "cas" migrations. We might want
        // to think about restructuring this
        self.cas.cache.pg().migrate().await?;

        Ok(())
    }
}

#[must_use = "graceful shutdown must be spawned on runtime"]
#[derive(Debug, Clone)]
pub struct LayerDbGracefulShutdown {
    tracker: TaskTracker,
    token: CancellationToken,
}

impl IntoFuture for LayerDbGracefulShutdown {
    type Output = io::Result<()>;
    type IntoFuture = private::GracefulShutdownFuture;

    fn into_future(self) -> Self::IntoFuture {
        let Self { token, tracker } = self;

        private::GracefulShutdownFuture(Box::pin(async move {
            // Wait until token is cancelled--this is our graceful shutdown signal
            token.cancelled().await;

            // Close the tracker so no further tasks are spawned
            tracker.close();
            trace!("received graceful shutdown signal, waiting for tasks to shutdown");
            // Wait for all outstanding tasks to complete
            tracker.wait().await;

            Ok(())
        }))
    }
}

mod private {
    use std::{
        fmt,
        future::Future,
        io,
        pin::Pin,
        task::{Context, Poll},
    };

    pub struct GracefulShutdownFuture(
        pub(super) futures::future::BoxFuture<'static, io::Result<()>>,
    );

    impl Future for GracefulShutdownFuture {
        type Output = io::Result<()>;

        #[inline]
        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            self.0.as_mut().poll(cx)
        }
    }

    impl fmt::Debug for GracefulShutdownFuture {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("ShutdownFuture").finish_non_exhaustive()
        }
    }
}
