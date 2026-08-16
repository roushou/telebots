//! The free-form message handler: answers text messages that weren't
//! consumed by a command branch.

use anyhow::Result;

use crate::{reply::Reply, request::MessageRequest};

/// The behavior a bot implements for free-form text messages.
#[crate::async_trait]
pub trait MessageHandler: Clone + Send + Sync + 'static {
    /// Everything the handler needs to produce its reply.
    type Ctx: Clone + Send + Sync + 'static;

    /// Produce the reply for this message, or `None` to stay silent (for
    /// example, group chatter that is neither an @mention nor a reply).
    ///
    /// Errors are authored with `anyhow`; botkit transports and renders them
    /// (`⚠️ {e:#}`).
    async fn handle(&self, ctx: &Self::Ctx, req: &MessageRequest) -> Result<Option<Reply>>;
}
