//! The reply model: what a command wants the bot to do.
//!
//! [`Reply::Background`] jobs carry [`anyhow::Result`] because command
//! errors are authored with `anyhow` in the binaries; botkit only
//! transports and renders them (`⚠️ {e:#}`). Execution lives in
//! [`crate::runtime::dispatch`].

use std::{future::Future, pin::Pin, time::Duration};

use anyhow::Result;
use telebots_core::Block;

use crate::markup::Markup;

/// Telegram's photo caption length limit.
const MAX_CAPTION_LEN: usize = 1024;

/// A boxed, sendable future.
pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

/// What a command wants the bot to do. Interpreted by botkit's single send
/// point — commands never call `send_message`.
#[non_exhaustive]
pub enum Reply {
    /// Send this block as a text message, optionally with an inline
    /// keyboard (capped at 4096).
    Text {
        block: Block,
        markup: Option<Markup>,
    },

    /// Deliver a photo with an optional caption and keyboard (capped at
    /// 1024).
    Photo {
        bytes: Vec<u8>,
        caption: Option<String>,
        markup: Option<Markup>,
    },

    /// Edit the acknowledgement placeholder in place (background jobs
    /// only). In the direct path there is nothing to edit, so it falls back
    /// to a normal text message.
    Edit(Block),

    /// Acknowledge with `placeholder`, run `job` in the background under
    /// supervision, then deliver its reply (or a uniform `⚠️` error).
    Background { placeholder: String, job: Job },
}

impl Reply {
    /// A text reply.
    pub fn text(block: Block) -> Self {
        Self::Text {
            block,
            markup: None,
        }
    }

    /// A photo reply.
    pub fn photo(bytes: Vec<u8>, caption: Option<String>) -> Self {
        Self::Photo {
            bytes,
            caption,
            markup: None,
        }
    }

    /// Attach an inline keyboard to a text or photo reply.
    pub fn with_markup(self, markup: Markup) -> Self {
        match self {
            Self::Text { block, .. } => Self::Text {
                block,
                markup: Some(markup),
            },
            Self::Photo { bytes, caption, .. } => Self::Photo {
                bytes,
                caption,
                markup: Some(markup),
            },
            other => other,
        }
    }

    /// Cap a caption at Telegram's limit.
    pub(crate) fn cap_caption(caption: String) -> String {
        if caption.chars().count() > MAX_CAPTION_LEN {
            let mut out: String = caption.chars().take(MAX_CAPTION_LEN - 1).collect();
            out.push('…');
            out
        } else {
            caption
        }
    }
}

/// A background job: what to run, and how long to let it run.
pub struct Job {
    pub timeout: Duration,
    pub(crate) run: Box<dyn FnOnce(JobCtx) -> BoxFuture<Result<Reply>> + Send>,
}

impl Job {
    /// A job that must finish within `timeout`; `run` produces the reply.
    pub fn new(
        timeout: Duration,
        run: impl FnOnce(JobCtx) -> BoxFuture<Result<Reply>> + Send + 'static,
    ) -> Self {
        Self {
            timeout,
            run: Box::new(run),
        }
    }
}

/// Everything a background job gets to finish the interaction.
pub struct JobCtx {
    pub chat_id: i64,
    pub user_id: Option<i64>,
    /// Report LLM token usage and cost into the bot's metrics.
    pub usage: crate::runtime::UsageReporter,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caption_capped_at_telegram_limit() {
        let long = "x".repeat(MAX_CAPTION_LEN + 10);
        let capped = Reply::cap_caption(long);
        assert!(capped.chars().count() <= MAX_CAPTION_LEN);
        assert!(capped.ends_with('…'));

        let short = "ok".to_string();
        assert_eq!(Reply::cap_caption(short), "ok");
    }

    #[test]
    fn text_can_carry_markup() {
        let mut block = Block::new();
        block.line("hi");
        let markup = Markup::new().row([crate::markup::Button::callback("Go", "go")]);
        let reply = Reply::text(block).with_markup(markup);
        match reply {
            Reply::Text {
                markup: Some(markup),
                ..
            } => assert!(!markup.is_empty()),
            _ => panic!("expected text reply with markup"),
        }
    }
}
