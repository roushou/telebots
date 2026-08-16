//! Callback-query support: button taps on inline keyboards.

use anyhow::Result;

use crate::{reply::Reply, request::CallbackRequest};

/// The behavior a bot implements for button taps.
#[crate::async_trait]
pub trait CallbackHandler: Clone + Send + Sync + 'static {
    /// Everything the handler needs to produce its reply.
    type Ctx: Clone + Send + Sync + 'static;

    /// Produce the reply for this tap: an [`Reply::Edit`] edits the message
    /// the button was on, anything else is sent to the chat.
    async fn handle(&self, ctx: &Self::Ctx, req: &CallbackRequest) -> Result<Reply>;
}
