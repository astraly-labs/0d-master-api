use crate::errors::DatabaseError;
use deadpool_diesel::postgres::Pool;

/// Extension trait for deadpool-diesel Pool to provide cleaner error handling
pub trait ZerodPool {
    /// Interact with the database with automatic error handling and logging
    ///
    /// # Arguments
    /// * `operation` - A description of the operation for logging context
    /// * `f` - The database operation to perform
    ///
    /// # Example
    /// ```
    /// let vaults = pool
    ///     .interact_with_context("fetch all vaults", Vault::find_all)
    ///     .await?;
    /// ```
    fn interact_with_context<F, T, E>(
        &self,
        operation: String,
        f: F,
    ) -> impl std::future::Future<Output = Result<T, DatabaseError>> + Send
    where
        F: FnOnce(&mut diesel::PgConnection) -> Result<T, E> + Send + 'static,
        T: Send + 'static,
        E: Into<DatabaseError> + Send + 'static;
}

impl ZerodPool for Pool {
    async fn interact_with_context<F, T, E>(
        &self,
        operation: String,
        f: F,
    ) -> Result<T, DatabaseError>
    where
        F: FnOnce(&mut diesel::PgConnection) -> Result<T, E> + Send + 'static,
        T: Send + 'static,
        E: Into<DatabaseError> + Send + 'static,
    {
        // Get connection from pool
        let conn = self.get().await.map_err(|e| {
            tracing::error!(
                operation = %operation,
                error = %e,
                "Failed to get database connection from pool"
            );
            DatabaseError::PoolError {
                operation: operation.clone(),
                message: e.to_string(),
            }
        })?;

        // Execute the database operation
        conn.interact(move |conn| f(conn))
            .await
            .map_err(|e| {
                tracing::error!(
                    operation = %operation,
                    error = %e,
                    "Database interaction failed (deadpool error)"
                );
                DatabaseError::InteractionError {
                    operation: operation.clone(),
                    message: e.to_string(),
                }
            })?
            .map_err(|e| {
                let db_error: DatabaseError = e.into();
                tracing::error!(
                    operation = %operation,
                    error = %db_error,
                    "Database query failed"
                );
                db_error
            })
    }
}
