//! Per-user rate limiting for free-form chat.

use anyhow::Result;

use crate::store::Store;

/// Minimum seconds between two chat requests from the same user.
pub const COOLDOWN_SECS: i64 = 10;

/// Reject chat requests that come too close to the previous one. The
/// cooldown lives in the persistent store, so it survives restarts.
#[derive(Clone)]
pub struct Cooldown;

impl Cooldown {
    /// Seconds the user must still wait, when they're cooling down.
    pub async fn remaining(
        &self,
        store: &Store,
        chat_id: i64,
        user_id: i64,
    ) -> Result<Option<i64>> {
        let now = telebots_core::Time::now_secs();
        if let Some(last) = store.cooldown(chat_id, user_id).await?
            && now - last < COOLDOWN_SECS
        {
            return Ok(Some(COOLDOWN_SECS - (now - last)));
        }
        Ok(None)
    }

    /// Record a use for this user.
    pub async fn record(&self, store: &Store, chat_id: i64, user_id: i64) -> Result<()> {
        store
            .set_cooldown(chat_id, user_id, telebots_core::Time::now_secs())
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn store() -> Store {
        Store::open(":memory:").await.unwrap()
    }

    #[tokio::test]
    async fn first_request_passes() -> Result<()> {
        let store = store().await;
        assert_eq!(Cooldown.remaining(&store, 1, 42).await?, None);
        Ok(())
    }

    #[tokio::test]
    async fn second_request_is_blocked() -> Result<()> {
        let store = store().await;
        Cooldown.record(&store, 1, 42).await?;
        let wait = Cooldown.remaining(&store, 1, 42).await?;
        assert!(wait.is_some());
        assert!(wait.unwrap() > 0);
        Ok(())
    }

    #[tokio::test]
    async fn cooldown_is_per_user() -> Result<()> {
        let store = store().await;
        Cooldown.record(&store, 1, 42).await?;
        assert_eq!(Cooldown.remaining(&store, 1, 7).await?, None);
        Ok(())
    }
}
