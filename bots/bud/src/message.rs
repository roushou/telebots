//! Free-form chat: the message handler that answers any text message bud
//! should respond to (always in private chats; only when @mentioned or
//! replied to in groups).

use std::time::Duration;

use anyhow::Result;
use botkit::{ChatKind, Job, MessageHandler, MessageRequest, Reply};
use telebots_core::Block;

use crate::{
    commands::Ctx,
    conversation::Conversation,
    cooldown::{COOLDOWN_SECS, Cooldown},
    pricing::Pricing,
    render,
};

/// How long a generation may take before the bot gives up.
const JOB_TIMEOUT: Duration = Duration::from_secs(180);

/// The free-form chat handler.
#[derive(Clone)]
pub struct Chat;

impl Chat {
    /// Whether to answer this message.
    fn should_respond(req: &MessageRequest) -> bool {
        match req.chat_kind {
            ChatKind::Private => true,
            ChatKind::Group | ChatKind::Supergroup => req.mentioned || req.replied_to_bot,
            ChatKind::Channel | _ => false,
        }
    }
}

#[botkit::async_trait]
impl MessageHandler for Chat {
    type Ctx = Ctx;

    async fn handle(&self, ctx: &Ctx, req: &MessageRequest) -> Result<Option<Reply>> {
        if !Self::should_respond(req) {
            return Ok(None);
        }

        // Rate-limit per user (persistent, survives restarts).
        if let Some(user_id) = req.user_id {
            if let Some(wait) = Cooldown
                .remaining(&ctx.storage, req.chat_id, user_id)
                .await?
            {
                let mut b = Block::new();
                b.line(format!(
                    "⏳ one message every {COOLDOWN_SECS}s — try again in {wait}s"
                ));
                return Ok(Some(Reply::text(b)));
            }
            Cooldown.record(&ctx.storage, req.chat_id, user_id).await?;
        }

        // Persist the user message, then build context (history now ends
        // with it).
        ctx.storage
            .add_message(req.chat_id, req.user_id, "user", &req.text)
            .await?;
        let history = ctx
            .storage
            .recent_messages(req.chat_id, ctx.max_history)
            .await?;

        let settings = ctx.storage.settings(req.chat_id).await?;
        let system_prompt = settings
            .system_prompt
            .clone()
            .unwrap_or_else(|| ctx.default_system_prompt.clone());
        let messages = Conversation::build(&system_prompt, &history);

        let generator = ctx.generator.clone();
        let storage = ctx.storage.clone();
        let model = settings.model;

        Ok(Some(Reply::Background {
            placeholder: "✍️ thinking…".to_string(),
            job: Job::new(JOB_TIMEOUT, move |job| {
                Box::pin(async move {
                    let completion = generator.chat(model, &messages).await?;
                    if let Some(usage) = &completion.usage {
                        job.usage.report(
                            usage.prompt_tokens,
                            usage.completion_tokens,
                            Pricing::cost_micro_usd(model, usage),
                        );
                    }
                    if let Err(e) = storage
                        .add_message(job.chat_id, job.user_id, "assistant", &completion.text)
                        .await
                    {
                        tracing::warn!("failed to record assistant reply: {e:#}");
                    }
                    Ok(Reply::Edit(render::answer(&completion.text)))
                })
            }),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn responds_always_in_private() {
        let req = MessageRequest::new("hi", 1, Some(42));
        assert!(Chat::should_respond(&req));
    }

    #[test]
    fn responds_in_group_only_when_addressed() {
        let mut base = MessageRequest::new("hi", 1, Some(42));
        base.chat_kind = ChatKind::Supergroup;
        assert!(!Chat::should_respond(&base));

        base.mentioned = true;
        assert!(Chat::should_respond(&base));

        base.mentioned = false;
        base.replied_to_bot = true;
        assert!(Chat::should_respond(&base));
    }

    #[test]
    fn never_responds_in_channel() {
        let mut req = MessageRequest::new("hi", 1, Some(42));
        req.chat_kind = ChatKind::Channel;
        req.mentioned = true;
        assert!(!Chat::should_respond(&req));
    }
}
