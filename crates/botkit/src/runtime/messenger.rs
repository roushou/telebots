//! The transport seam: botkit's own view of "deliver this message".
//! teloxide's `Bot` implements it; tests use a mock. Crate-private — bots
//! produce `Reply`s, they never touch this.

use teloxide::{
    Bot as Api,
    payloads::{SendMessageSetters, SendPhotoSetters},
    prelude::Requester,
    requests::ResponseResult,
    types::{CallbackQueryId, ChatId, InputFile, MessageId, ReplyMarkup, ReplyParameters},
};

use crate::{markup::Markup, reply::Reply};

/// The message operations the dispatcher needs.
#[crate::async_trait]
pub(crate) trait Messenger: Clone + Send + Sync + 'static {
    /// Send a text message (with an optional keyboard); returns its id.
    async fn send_text(
        &self,
        chat: ChatId,
        text: String,
        markup: Option<Markup>,
    ) -> ResponseResult<MessageId>;

    /// Send a photo replying to `reply_to` (with an optional keyboard);
    /// returns its id.
    async fn send_photo(
        &self,
        chat: ChatId,
        reply_to: MessageId,
        bytes: Vec<u8>,
        caption: Option<String>,
        markup: Option<Markup>,
    ) -> ResponseResult<MessageId>;

    /// Replace a message's text in place.
    async fn edit_text(&self, chat: ChatId, msg: MessageId, text: String) -> ResponseResult<()>;

    /// Delete a message.
    async fn delete(&self, chat: ChatId, msg: MessageId) -> ResponseResult<()>;

    /// Acknowledge a button tap (stops the loading spinner).
    async fn answer_callback(&self, query_id: CallbackQueryId) -> ResponseResult<()>;
}

#[crate::async_trait]
impl Messenger for Api {
    async fn send_text(
        &self,
        chat: ChatId,
        text: String,
        markup: Option<Markup>,
    ) -> ResponseResult<MessageId> {
        let mut request = self.send_message(chat, text);
        if let Some(markup) = markup {
            request = request.reply_markup(ReplyMarkup::InlineKeyboard(markup.into_telegram()));
        }
        let message = request.await?;
        Ok(message.id)
    }

    async fn send_photo(
        &self,
        chat: ChatId,
        reply_to: MessageId,
        bytes: Vec<u8>,
        caption: Option<String>,
        markup: Option<Markup>,
    ) -> ResponseResult<MessageId> {
        let mut request = Requester::send_photo(self, chat, InputFile::memory(bytes))
            .reply_parameters(ReplyParameters::new(reply_to));
        if let Some(caption) = caption {
            request = request.caption(Reply::cap_caption(caption));
        }
        if let Some(markup) = markup {
            request = request.reply_markup(ReplyMarkup::InlineKeyboard(markup.into_telegram()));
        }
        let message = request.await?;
        Ok(message.id)
    }

    async fn edit_text(&self, chat: ChatId, msg: MessageId, text: String) -> ResponseResult<()> {
        self.edit_message_text(chat, msg, text).await.map(|_| ())
    }

    async fn delete(&self, chat: ChatId, msg: MessageId) -> ResponseResult<()> {
        self.delete_message(chat, msg).await.map(|_| ())
    }

    async fn answer_callback(&self, query_id: CallbackQueryId) -> ResponseResult<()> {
        self.answer_callback_query(query_id).await.map(|_| ())
    }
}
