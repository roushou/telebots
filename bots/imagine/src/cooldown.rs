//! Per-user rate limiting for `/imagine`, applied as a router guard.

use anyhow::Result;
use botkit::{Guard, Reply, Request};
use telebots_core::Block;

use crate::commands::{Command, Ctx};

const COOLDOWN_SECS: i64 = 30;

/// Reject `/imagine` requests that come too close to the previous one. The
/// cooldown lives in the persistent store, so it survives restarts.
#[derive(Clone)]
pub struct Cooldown;

#[botkit::async_trait]
impl Guard<Command, Ctx> for Cooldown {
    async fn check(&self, ctx: &Ctx, req: &Request, cmd: &Command) -> Result<Option<Reply>> {
        if !matches!(cmd, Command::Imagine(_)) {
            return Ok(None);
        }
        let Some(user_id) = req.user_id else {
            return Ok(None);
        };

        let now = telebots_core::Time::now_secs();
        if let Some(last) = ctx.storage.cooldown(req.chat_id, user_id).await?
            && now - last < COOLDOWN_SECS
        {
            let wait = COOLDOWN_SECS - (now - last);
            let mut block = Block::new();
            block.line(format!(
                "⏳ one image every {COOLDOWN_SECS}s — try again in {wait}s"
            ));
            return Ok(Some(Reply::text(block)));
        }

        ctx.storage.set_cooldown(req.chat_id, user_id, now).await?;
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{commands::Ctx, generator::Generator, store::Store};

    async fn ctx() -> anyhow::Result<Ctx> {
        Ok(Ctx {
            generator: Generator::cloudflare("acct".into(), "tok".into())?,
            storage: Store::open(":memory:").await?,
        })
    }

    #[tokio::test]
    async fn blocks_repeated_requests() -> anyhow::Result<()> {
        let ctx = ctx().await?;
        let guard = Cooldown;
        let req = Request::new(1, Some(42));
        let cmd = Command::Imagine("a cat".into());
        assert!(guard.check(&ctx, &req, &cmd).await?.is_none());
        assert!(guard.check(&ctx, &req, &cmd).await?.is_some());
        Ok(())
    }

    #[tokio::test]
    async fn cooldown_is_per_user() -> anyhow::Result<()> {
        let ctx = ctx().await?;
        let guard = Cooldown;
        let cmd = Command::Imagine("a cat".into());
        assert!(
            guard
                .check(&ctx, &Request::new(1, Some(42)), &cmd)
                .await?
                .is_none()
        );
        assert!(
            guard
                .check(&ctx, &Request::new(1, Some(7)), &cmd)
                .await?
                .is_none()
        );
        Ok(())
    }

    #[tokio::test]
    async fn ignores_other_commands() -> anyhow::Result<()> {
        let ctx = ctx().await?;
        let guard = Cooldown;
        let req = Request::new(1, Some(42));
        assert!(guard.check(&ctx, &req, &Command::History).await?.is_none());
        assert!(guard.check(&ctx, &req, &Command::History).await?.is_none());
        Ok(())
    }
}
