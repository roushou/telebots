//! Command guards: run after parsing but before a command's `reply`, and
//! may short-circuit it with a reply of their own (rate limits, permissions,
//! registration, ...).

use anyhow::Result;

use crate::{Reply, Request};

/// A pre-command check. Return `Some(reply)` to stop the command and send
/// that reply instead, or `None` to let it proceed.
#[crate::async_trait]
pub trait Guard<C, Ctx>: Clone + Send + Sync + 'static
where
    C: Send + Sync + 'static,
    Ctx: Send + Sync + 'static,
{
    async fn check(&self, ctx: &Ctx, req: &Request, cmd: &C) -> Result<Option<Reply>>;
}

/// The guard used when a command branch has none: always proceeds.
#[derive(Clone, Copy, Default)]
pub struct NoGuard;

#[crate::async_trait]
impl<C, Ctx> Guard<C, Ctx> for NoGuard
where
    C: Send + Sync + 'static,
    Ctx: Send + Sync + 'static,
{
    async fn check(&self, _ctx: &Ctx, _req: &Request, _cmd: &C) -> Result<Option<Reply>> {
        Ok(None)
    }
}

/// Guards compose left-to-right: the first to short-circuit wins.
#[crate::async_trait]
impl<C, Ctx, A, B> Guard<C, Ctx> for (A, B)
where
    C: Send + Sync + 'static,
    Ctx: Send + Sync + 'static,
    A: Guard<C, Ctx>,
    B: Guard<C, Ctx>,
{
    async fn check(&self, ctx: &Ctx, req: &Request, cmd: &C) -> Result<Option<Reply>> {
        if let Some(reply) = self.0.check(ctx, req, cmd).await? {
            return Ok(Some(reply));
        }
        self.1.check(ctx, req, cmd).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct Block;

    #[crate::async_trait]
    impl<C: Send + Sync + 'static, Ctx: Send + Sync + 'static> Guard<C, Ctx> for Block {
        async fn check(&self, _ctx: &Ctx, _req: &Request, _cmd: &C) -> Result<Option<Reply>> {
            Ok(Some(Reply::Text(telebots_core::Block::new())))
        }
    }

    #[tokio::test]
    async fn no_guard_proceeds() {
        let req = Request::new(1, None);
        assert!(NoGuard.check(&(), &req, &()).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn tuples_short_circuit_left_to_right() {
        let req = Request::new(1, None);
        // First blocks: the second never runs.
        assert!(
            (Block, NoGuard)
                .check(&(), &req, &())
                .await
                .unwrap()
                .is_some()
        );
        // First proceeds: the second blocks.
        assert!(
            (NoGuard, Block)
                .check(&(), &req, &())
                .await
                .unwrap()
                .is_some()
        );
        // Neither blocks.
        assert!(
            (NoGuard, NoGuard)
                .check(&(), &req, &())
                .await
                .unwrap()
                .is_none()
        );
    }
}
