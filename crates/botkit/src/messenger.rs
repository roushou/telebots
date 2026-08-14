//! The transport seam: botkit's own view of "deliver this message".
//! teloxide's `Bot` implements it; tests use a mock. Crate-private — bots
//! produce `Reply`s, they never touch this.

use teloxide::{
    Bot as Api,
    payloads::SendPhotoSetters,
    prelude::Requester,
    requests::ResponseResult,
    types::{ChatId, InputFile, MessageId, ReplyParameters},
};

use crate::reply::Reply;

/// The message operations the dispatcher needs.
#[crate::async_trait]
pub(crate) trait Messenger: Clone + Send + Sync + 'static {
    /// Send a text message; returns its id.
    async fn send_text(&self, chat: ChatId, text: String) -> ResponseResult<MessageId>;

    /// Send a photo replying to `reply_to`; returns its id.
    async fn send_photo(
        &self,
        chat: ChatId,
        reply_to: MessageId,
        bytes: Vec<u8>,
        caption: Option<String>,
    ) -> ResponseResult<MessageId>;

    /// Replace a message's text in place.
    async fn edit_text(&self, chat: ChatId, msg: MessageId, text: String) -> ResponseResult<()>;

    /// Delete a message.
    async fn delete(&self, chat: ChatId, msg: MessageId) -> ResponseResult<()>;
}

#[crate::async_trait]
impl Messenger for Api {
    async fn send_text(&self, chat: ChatId, text: String) -> ResponseResult<MessageId> {
        let message = self.send_message(chat, text).await?;
        Ok(message.id)
    }

    async fn send_photo(
        &self,
        chat: ChatId,
        reply_to: MessageId,
        bytes: Vec<u8>,
        caption: Option<String>,
    ) -> ResponseResult<MessageId> {
        let mut request = Requester::send_photo(self, chat, InputFile::memory(bytes))
            .reply_parameters(ReplyParameters::new(reply_to));
        if let Some(caption) = caption {
            request = request.caption(Reply::cap_caption(caption));
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
}
