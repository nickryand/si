use std::sync::Arc;

use si_data::{
    nats,
    pg::{self, InstrumentedClient, InstrumentedTransaction, PgPoolResult},
    NatsClient, NatsTxn, PgPool, PgTxn,
};
use thiserror::Error;
use veritech::EncryptionKey;

use crate::{
    BillingAccountId, HistoryActor, OrganizationId, ReadTenancy, ReadTenancyError, Visibility,
    WorkspaceId, WriteTenancy,
};

/// A context type which contains handles to common core service dependencies.
///
/// These services are typically used by most DAL objects, such as a database connection pool, a
/// function execution client, etc.
#[derive(Clone, Debug)]
pub struct ServicesContext {
    /// A PostgreSQL connection pool.
    pg_pool: PgPool,
    /// A connected NATS client
    nats_conn: NatsClient,
    /// A Veritech client, connected via a NATS connection.
    veritech: veritech::Client,
    /// A key for re-recrypting messages to the function execution system.
    encryption_key: Arc<veritech::EncryptionKey>,
}

impl ServicesContext {
    /// Constructs a new instance of a `ServicesContext`.
    pub fn new(
        pg_pool: PgPool,
        nats_conn: NatsClient,
        veritech: veritech::Client,
        encryption_key: Arc<veritech::EncryptionKey>,
    ) -> Self {
        Self {
            pg_pool,
            nats_conn,
            veritech,
            encryption_key,
        }
    }

    /// Consumes and returns [`DalContextBuilder`].
    pub fn into_builder(self) -> DalContextBuilder {
        DalContextBuilder {
            services_context: self,
        }
    }

    /// Gets a reference to the Postgres pool.
    pub fn pg_pool(&self) -> &PgPool {
        &self.pg_pool
    }

    /// Gets a reference to the NATS connection.
    pub fn nats_conn(&self) -> &NatsClient {
        &self.nats_conn
    }

    /// Gets a reference to the Veritech client.
    pub fn veritech(&self) -> &veritech::Client {
        &self.veritech
    }

    /// Gets a reference to the encryption key.
    pub fn encryption_key(&self) -> &EncryptionKey {
        &self.encryption_key
    }

    /// Builds and returns a new [`TransactionsStarter`].
    pub async fn transactions_starter(&self) -> PgPoolResult<TransactionsStarter> {
        let pg_conn = self.pg_pool.get().await?;
        let nats_conn = self.nats_conn.clone();
        Ok(TransactionsStarter::new(pg_conn, nats_conn))
    }
}

/// A context type which holds references to underlying services, transactions, and read/write
/// context for DAL objects.
#[derive(Clone, Debug)]
pub struct DalContext<'s, 't> {
    /// A reference to a [`ServicesContext`] which has handles to common core services.
    services_context: &'s ServicesContext,
    /// A reference to a set of atomically related transactions.
    txns: &'t Transactions<'t>,
    /// A suitable read tenancy for the consuming DAL objects.
    read_tenancy: ReadTenancy,
    /// A suitable write tenancy for the consuming DAL objects.
    write_tenancy: WriteTenancy,
    /// A suitable [`Visibility`] scope for the consuming DAL objects.
    visibility: Visibility,
    /// A suitable [`HistoryActor`] for the consuming DAL objects.
    history_actor: HistoryActor,
}

impl DalContext<'_, '_> {
    /// Takes a reference to a [`ServicesContext`] and returns a builder to construct a
    /// `DalContext`.
    pub fn builder(services_context: ServicesContext) -> DalContextBuilder {
        DalContextBuilder { services_context }
    }

    /// Updates this context with a new [`Visibility`].
    pub fn update_visibility(&mut self, visibility: Visibility) {
        self.visibility = visibility;
    }

    /// Clones a new context from this one with a new [`Visibility`].
    pub fn clone_with_new_visibility(&self, visibility: Visibility) -> Self {
        let mut new = self.clone();
        new.update_visibility(visibility);
        new
    }

    /// Updates this context with a new [`ReadTenancy`] and [`WriteTenancy`].
    pub fn update_tenancies(&mut self, read_tenancy: ReadTenancy, write_tenancy: WriteTenancy) {
        self.read_tenancy = read_tenancy;
        self.write_tenancy = write_tenancy;
    }

    /// Clones a new context from this one with a new [`ReadTenancy`] and [`WriteTenancy`].
    pub fn clone_with_new_tenancies(
        &self,
        read_tenancy: ReadTenancy,
        write_tenancy: WriteTenancy,
    ) -> Self {
        let mut new = self.clone();
        new.update_tenancies(read_tenancy, write_tenancy);
        new
    }

    /// Updates this context with a new [`WriteTenancy`].
    pub fn update_write_tenancy(&mut self, write_tenancy: WriteTenancy) {
        self.write_tenancy = write_tenancy;
    }

    /// Clones a new context from this one with a new [`WriteTenancy`].
    pub fn clone_with_new_write_tenancy(&self, write_tenancy: WriteTenancy) -> Self {
        let mut new = self.clone();
        new.update_write_tenancy(write_tenancy);
        new
    }

    /// Updates this context with a new [`ReadTenancy`].
    pub fn update_read_tenancy(&mut self, read_tenancy: ReadTenancy) {
        self.read_tenancy = read_tenancy;
    }

    /// Clones a new context from this one with a new [`ReadTenancy`].
    pub fn clone_with_new_read_tenancy(&self, read_tenancy: ReadTenancy) -> Self {
        let mut new = self.clone();
        new.update_read_tenancy(read_tenancy);
        new
    }

    /// Updates this context with universal-scoped read/write tenancies and a head [`Visibility`].
    pub fn update_to_universal_head(&mut self) {
        self.read_tenancy = ReadTenancy::new_universal();
        self.write_tenancy = WriteTenancy::new_universal();
        self.visibility = Visibility::new_head(false);
    }

    /// Clones a new context from this one with universal-scoped read/write tenancies and a head
    /// [`Visibility`].
    pub fn clone_with_universal_head(&self) -> Self {
        let mut new = self.clone();
        new.update_to_universal_head();
        new
    }

    /// Updates this context with read/write tenancies for a specific billing account.
    pub fn update_to_billing_account_tenancies(&mut self, bid: BillingAccountId) {
        self.read_tenancy = ReadTenancy::new_billing_account(vec![bid]);
        self.write_tenancy = WriteTenancy::new_billing_account(bid);
    }

    /// Clones a new context from this one with read/write tenancies for a specific billing account.
    pub fn clone_with_new_billing_account_tenancies(&self, bid: BillingAccountId) -> Self {
        let mut new = self.clone();
        new.update_to_billing_account_tenancies(bid);
        new
    }

    /// Updates this context with read/write tenancies for a specific organization.
    pub async fn update_to_organization_tenancies(
        &mut self,
        pg_txn: &PgTxn<'_>,
        oid: OrganizationId,
    ) -> Result<(), TransactionsError> {
        self.read_tenancy = ReadTenancy::new_organization(pg_txn, vec![oid]).await?;
        self.write_tenancy = WriteTenancy::new_organization(oid);
        Ok(())
    }

    /// Clones a new context from this one with read/write tenancies for a specific organization.
    pub async fn clone_with_new_organization_tenancies(
        &self,
        pg_txn: &PgTxn<'_>,
        oid: OrganizationId,
    ) -> Result<DalContext<'_, '_>, TransactionsError> {
        let mut new = self.clone();
        new.update_to_organization_tenancies(pg_txn, oid).await?;
        Ok(new)
    }

    /// Updates this context with read/write tenancies for a specific workspace.
    pub async fn update_to_workspace_tenancies(
        &mut self,
        pg_txn: &PgTxn<'_>,
        wid: WorkspaceId,
    ) -> Result<(), TransactionsError> {
        self.read_tenancy = ReadTenancy::new_workspace(pg_txn, vec![wid]).await?;
        self.write_tenancy = WriteTenancy::new_workspace(wid);
        Ok(())
    }

    /// Clones a new context from this one with read/write tenancies for a specific workspace.
    pub async fn clone_with_new_workspace_tenancies(
        &self,
        pg_txn: &PgTxn<'_>,
        wid: WorkspaceId,
    ) -> Result<DalContext<'_, '_>, TransactionsError> {
        let mut new = self.clone();
        new.update_to_workspace_tenancies(pg_txn, wid).await?;
        Ok(new)
    }

    /// Gets the dal context's txns.
    pub fn txns(&self) -> &Transactions {
        self.txns
    }

    /// Gets a reference to the DAL context's Postgres pool.
    pub fn pg_pool(&self) -> &PgPool {
        &self.services_context.pg_pool
    }

    /// Gets the dal context's pg txn.
    pub fn pg_txn(&self) -> &InstrumentedTransaction {
        &self.txns.pg_txn
    }

    /// Gets a reference to the DAL context's NATS connection.
    pub fn nats_conn(&self) -> &NatsClient {
        &self.services_context.nats_conn
    }

    /// Gets the dal context's nats txn.
    pub fn nats_txn(&self) -> &NatsTxn {
        &self.txns.nats_txn
    }

    /// Gets a reference to the DAL context's Veritech client.
    pub fn veritech(&self) -> &veritech::Client {
        &self.services_context.veritech
    }

    /// Gets a reference to the DAL context's encryption key.
    pub fn encryption_key(&self) -> &EncryptionKey {
        &self.services_context.encryption_key
    }

    /// Gets a reference to the dal context's read tenancy.
    pub fn read_tenancy(&self) -> &ReadTenancy {
        &self.read_tenancy
    }

    /// Gets a reference to the dal context's write tenancy.
    pub fn write_tenancy(&self) -> &WriteTenancy {
        &self.write_tenancy
    }

    /// Gets the dal context's visibility.
    pub fn visibility(&self) -> &Visibility {
        &self.visibility
    }

    /// Gets a reference to the dal context's history actor.
    pub fn history_actor(&self) -> &HistoryActor {
        &self.history_actor
    }
}

/// A context which represents a suitable tenancies, visibilities, etc. for consumption by a set
/// of DAL objects.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequestContext {
    /// A suitable read tenancy for the consuming DAL objects.
    pub read_tenancy: ReadTenancy,
    /// A suitable write tenancy for the consuming DAL objects.
    pub write_tenancy: WriteTenancy,
    /// A suitable [`Visibility`] scope for the consuming DAL objects.
    pub visibility: Visibility,
    /// A suitable [`HistoryActor`] for the consuming DAL objects.
    pub history_actor: HistoryActor,
}

impl RequestContext {
    /// Builds a new [`RequestContext`] with universal read/write tenancies and a head
    /// [`Visibility`] and the given [`HistoryActor`].
    pub fn new_universal_head(history_actor: HistoryActor) -> Self {
        let visibility = Visibility::new_head(false);
        let read_tenancy = ReadTenancy::new_universal();
        let write_tenancy = WriteTenancy::new_universal();
        Self {
            read_tenancy,
            write_tenancy,
            visibility,
            history_actor,
        }
    }

    /// Builds a new [`RequestContext`] with read/write tenancies for a specific workspace and a
    /// head [`Visibility`] and the given [`HistoryActor`].
    pub async fn new_workspace_head(
        pg_txn: &PgTxn<'_>,
        history_actor: HistoryActor,
        workspace_id: WorkspaceId,
    ) -> Result<Self, TransactionsError> {
        let read_tenancy = ReadTenancy::new_workspace(pg_txn, vec![workspace_id]).await?;
        let write_tenancy = WriteTenancy::new_workspace(workspace_id);
        let visibility = Visibility::new_head(false);

        Ok(Self {
            read_tenancy,
            write_tenancy,
            visibility,
            history_actor,
        })
    }

    /// Get a reference to the request context's read tenancy.
    #[must_use]
    pub fn read_tenancy(&self) -> &ReadTenancy {
        &self.read_tenancy
    }

    /// Get a reference to the request context's write tenancy.
    #[must_use]
    pub fn write_tenancy(&self) -> &WriteTenancy {
        &self.write_tenancy
    }

    /// Get the request context's visibility.
    #[must_use]
    pub fn visibility(&self) -> &Visibility {
        &self.visibility
    }

    /// Get a reference to the request context's history actor.
    #[must_use]
    pub fn history_actor(&self) -> &HistoryActor {
        &self.history_actor
    }
}

/// A request context builder which requires a [`Visibility`] to be completed.
#[derive(Debug)]
pub struct AccessBuilder {
    /// A suitable read tenancy for the consuming DAL objects.
    read_tenancy: ReadTenancy,
    /// A suitable write tenancy for the consuming DAL objects.
    write_tenancy: WriteTenancy,
    /// A suitable [`HistoryActor`] for the consuming DAL objects.
    history_actor: HistoryActor,
}

impl AccessBuilder {
    /// Constructs a new instance given a set of tenancies and a [`HistoryActor`].
    pub fn new(
        read_tenancy: ReadTenancy,
        write_tenancy: WriteTenancy,
        history_actor: HistoryActor,
    ) -> Self {
        Self {
            read_tenancy,
            write_tenancy,
            history_actor,
        }
    }

    /// Builds and returns a new [`RequestContext`] using the default [`Visibility`].
    pub fn build_head(self) -> RequestContext {
        RequestContext {
            read_tenancy: self.read_tenancy,
            write_tenancy: self.write_tenancy,
            visibility: Visibility::new_head(false),
            history_actor: self.history_actor,
        }
    }

    /// Builds and returns a new [`RequestContext`] using the given [`Visibility`].
    pub fn build(self, visibility: Visibility) -> RequestContext {
        RequestContext {
            read_tenancy: self.read_tenancy,
            write_tenancy: self.write_tenancy,
            visibility,
            history_actor: self.history_actor,
        }
    }
}

/// A builder for a [`DalContext`].
pub struct DalContextBuilder {
    /// A [`ServicesContext`] which has handles to common core services.
    services_context: ServicesContext,
}

impl DalContextBuilder {
    /// Contructs and returns a new [`DalContext`] using the given transaction references and
    /// [`RequestContext`].
    pub fn build<'t>(
        &self,
        request_context: RequestContext,
        txns: &'t Transactions,
    ) -> DalContext<'_, 't> {
        DalContext {
            services_context: &self.services_context,
            txns,
            read_tenancy: request_context.read_tenancy,
            write_tenancy: request_context.write_tenancy,
            visibility: request_context.visibility,
            history_actor: request_context.history_actor,
        }
    }

    /// Gets a reference to the PostgreSQL connection pool.
    pub fn pg_pool(&self) -> &PgPool {
        &self.services_context.pg_pool
    }

    /// Gets a reference to the NATS connection.
    pub fn nats_conn(&self) -> &NatsClient {
        &self.services_context.nats_conn
    }

    /// Builds and returns a new [`TransactionsStarter`].
    pub async fn transactions_starter(&self) -> PgPoolResult<TransactionsStarter> {
        self.services_context.transactions_starter().await
    }
}

#[derive(Debug, Error)]
pub enum TransactionsError {
    #[error(transparent)]
    Pg(#[from] pg::Error),
    #[error(transparent)]
    Nats(#[from] nats::Error),
    #[error(transparent)]
    ReadTenancy(#[from] ReadTenancyError),
}

/// A type which holds ownership over connections that can be used to start transactions.
#[derive(Debug)]
pub struct TransactionsStarter {
    pg_conn: InstrumentedClient,
    nats_conn: NatsClient,
}

impl TransactionsStarter {
    /// Builds a new [`TransactionsStarter`].
    #[must_use]
    pub fn new(pg_conn: InstrumentedClient, nats_conn: NatsClient) -> Self {
        Self { pg_conn, nats_conn }
    }

    /// Starts and returns the underlying transactions as a [`Transactions`].
    pub async fn start(&mut self) -> Result<Transactions<'_>, TransactionsError> {
        let pg_txn = self.pg_conn.transaction().await?;
        let nats_txn = self.nats_conn.transaction();
        Ok(Transactions::new(pg_txn, nats_txn))
    }

    /// Gets a reference to a PostgreSQL connection.
    pub fn pg_conn(&self) -> &InstrumentedClient {
        &self.pg_conn
    }

    /// Gets a reference to a NATS connection.
    pub fn nats_conn(&self) -> &NatsClient {
        &self.nats_conn
    }
}

// A set of atomically-related transactions.
//
// Ideally, all of these inner transactions would be committed or rolled back together, hence the
// API methods.
#[derive(Debug)]
pub struct Transactions<'a> {
    /// A PostgreSQL transaction.
    pg_txn: PgTxn<'a>,
    /// A NATS transaction.
    nats_txn: NatsTxn,
}

impl<'a> Transactions<'a> {
    /// Creates and returns a new `Transactions` instance.
    pub fn new(pg_txn: PgTxn<'a>, nats_txn: NatsTxn) -> Self {
        Self { pg_txn, nats_txn }
    }

    /// Gets a reference to the PostgreSQL transaction.
    pub fn pg(&self) -> &PgTxn<'a> {
        &self.pg_txn
    }

    /// Gets a reference to the NATS transaction.
    pub fn nats(&self) -> &NatsTxn {
        &self.nats_txn
    }

    /// Consumes all inner transactions and committing all changes made within them.
    pub async fn commit(self) -> Result<(), TransactionsError> {
        self.pg_txn.commit().await?;
        self.nats_txn.commit().await?;
        Ok(())
    }

    /// Rolls all inner transactions back, discarding all changes made within them.
    ///
    /// This is equivalent to the transaction's `Drop` implementations, but provides any error
    /// encountered to the caller.
    pub async fn rollback(self) -> Result<(), TransactionsError> {
        self.pg_txn.rollback().await?;
        self.nats_txn.rollback().await?;
        Ok(())
    }
}
