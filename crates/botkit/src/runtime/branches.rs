//! The dptree branches: how each update kind is matched, parsed, and routed
//! to its handler, plus the endpoints that run the handlers and dispatch
//! their replies.

use teloxide::{
    Bot as Api, RequestError,
    dispatching::{UpdateFilterExt as _, UpdateHandler},
    prelude::Requester,
    requests::ResponseResult,
    types::{CallbackQuery, ChatId, InlineQuery, Me, Message, MessageId, Update},
};

use crate::{
    guard::Guard,
    handlers::{CallbackHandler, Command, InlineHandler, InlineResult, MessageHandler},
    reply::Reply,
    request::{CallbackRequest, InlineRequest, MessageRequest, Request},
    runtime::{MAX_MESSAGE_LEN, Messenger, Supervisor, dispatch},
};

/// The command branch: parse `/command` text, then run the guard and the
/// command.
pub(crate) fn command_branch<C, G>(guard: G) -> UpdateHandler<RequestError>
where
    C: Command,
    G: Guard<C, C::Ctx>,
{
    Update::filter_message()
        .filter_map(|msg: Message, me: Me| {
            let bot_name = me.user.username.as_deref()?;
            let text = msg.text().or_else(|| msg.caption())?;
            C::parse(text, bot_name)
        })
        .endpoint(
            move |cmd: C, bot: Api, msg: Message, ctx: C::Ctx, supervisor: Supervisor| {
                let guard = guard.clone();
                Box::pin(async move { handle_command(cmd, bot, msg, ctx, supervisor, guard).await })
            },
        )
}

/// The inline branch: answer `@botname <query>` with the handler's results.
pub(crate) fn inline_branch<I>(handler: I) -> UpdateHandler<RequestError>
where
    I: InlineHandler,
{
    Update::filter_inline_query().endpoint(move |query: InlineQuery, bot: Api, ctx: I::Ctx| {
        let handler = handler.clone();
        Box::pin(async move {
            let req = InlineRequest::from_query(&query);
            let results = match handler.handle(&ctx, &req).await {
                Ok(answer) => answer
                    .results
                    .into_iter()
                    .map(InlineResult::into_telegram)
                    .collect::<Vec<_>>(),
                Err(e) => {
                    tracing::warn!("inline query failed: {e:#}");
                    Vec::new()
                }
            };
            if let Err(e) = bot.answer_inline_query(query.id, results).await {
                tracing::warn!("failed to answer inline query: {e}");
            }
            Ok(())
        })
    })
}

/// The callback branch: handle inline-keyboard button taps.
pub(crate) fn callback_branch<H>(handler: H) -> UpdateHandler<RequestError>
where
    H: CallbackHandler,
{
    Update::filter_callback_query().endpoint(
        move |query: CallbackQuery, bot: Api, ctx: H::Ctx, supervisor: Supervisor| {
            let handler = handler.clone();
            Box::pin(async move { handle_callback(query, bot, ctx, supervisor, handler).await })
        },
    )
}

/// The message branch: turn any text message into a [`MessageRequest`] and
/// hand it to the handler. Non-text messages and messages without a bot
/// mention fall through to later branches (none by default).
pub(crate) fn message_branch<H>(handler: H) -> UpdateHandler<RequestError>
where
    H: MessageHandler,
{
    Update::filter_message()
        .filter_map(|msg: Message, me: Me| {
            let text = msg.text().or_else(|| msg.caption())?.to_string();
            let mentioned = me
                .user
                .username
                .as_deref()
                .map(|name| {
                    text.to_lowercase()
                        .contains(&format!("@{name}").to_lowercase())
                })
                .unwrap_or(false);
            let replied_to_bot = msg
                .reply_to_message()
                .and_then(|reply| reply.from.as_ref())
                .map(|user| user.id == me.user.id)
                .unwrap_or(false);
            let req = MessageRequest {
                request: Request::from_message(&msg),
                text,
                mentioned,
                replied_to_bot,
            };
            Some(req)
        })
        .endpoint(
            move |req: MessageRequest,
                  bot: Api,
                  msg: Message,
                  ctx: H::Ctx,
                  supervisor: Supervisor| {
                let handler = handler.clone();
                Box::pin(
                    async move { handle_message(req, bot, msg, ctx, supervisor, handler).await },
                )
            },
        )
}

/// The message endpoint: run the handler and dispatch its reply (or silence,
/// or a uniform error).
async fn handle_message<H>(
    req: MessageRequest,
    bot: Api,
    msg: Message,
    ctx: H::Ctx,
    supervisor: Supervisor,
    handler: H,
) -> ResponseResult<()>
where
    H: MessageHandler,
{
    let chat = msg.chat.id;
    let reply_to = msg.id;
    let user_id = msg.from.as_ref().map(|u| u.id.0 as i64);
    match handler.handle(&ctx, &req).await {
        Ok(Some(reply)) => {
            supervisor.note_command_named("message");
            dispatch(&bot, chat, reply_to, user_id, &supervisor, async {
                Ok(reply)
            })
            .await
        }
        Ok(None) => Ok(()),
        Err(e) => {
            supervisor.note_command_named("message");
            supervisor.note_command_error("message");
            dispatch(&bot, chat, reply_to, user_id, &supervisor, async { Err(e) }).await
        }
    }
}

/// The callback endpoint: run the handler, edit or send its reply, and
/// always acknowledge the tap.
async fn handle_callback<H>(
    query: CallbackQuery,
    bot: Api,
    ctx: H::Ctx,
    supervisor: Supervisor,
    handler: H,
) -> ResponseResult<()>
where
    H: CallbackHandler,
{
    let req = CallbackRequest::from_query(&query);
    let (Some(chat_id), Some(message_id)) = (req.chat_id, req.message_id) else {
        // Button on an inaccessible/inline message; just acknowledge.
        let _ = Messenger::answer_callback(&bot, query.id).await;
        return Ok(());
    };
    let chat = ChatId(chat_id);
    let msg = MessageId(message_id);

    match handler.handle(&ctx, &req).await {
        Ok(Reply::Edit(block)) => {
            if let Err(e) =
                Messenger::edit_text(&bot, chat, msg, block.truncate(MAX_MESSAGE_LEN).build()).await
            {
                tracing::warn!("failed to edit callback message: {e}");
            }
        }
        Ok(reply) => {
            dispatch(&bot, chat, msg, req.user_id, &supervisor, async {
                Ok(reply)
            })
            .await?;
        }
        Err(e) => {
            tracing::warn!("callback failed: {e:#}");
            let _ = Messenger::send_text(&bot, chat, format!("⚠️ {e:#}"), None).await;
        }
    }
    let _ = Messenger::answer_callback(&bot, query.id).await;
    Ok(())
}

/// The command endpoint: run the guard, then route either its reply or the
/// command's through botkit's send point.
async fn handle_command<C, G>(
    cmd: C,
    bot: Api,
    msg: Message,
    ctx: C::Ctx,
    supervisor: Supervisor,
    guard: G,
) -> ResponseResult<()>
where
    C: Command,
    G: Guard<C, C::Ctx>,
{
    let req = Request::from_message(&msg);
    let chat = msg.chat.id;
    let reply_to = msg.id;
    let user_id = msg.from.as_ref().map(|u| u.id.0 as i64);
    match guard.check(&ctx, &req, &cmd).await {
        Ok(Some(reply)) => {
            dispatch(&bot, chat, reply_to, user_id, &supervisor, async {
                Ok(reply)
            })
            .await
        }
        Ok(None) => {
            let name = cmd.name();
            supervisor.note_command_named(name);
            let result = cmd.reply(&ctx, &req).await;
            if result.is_err() {
                supervisor.note_command_error(name);
            }
            dispatch(&bot, chat, reply_to, user_id, &supervisor, async { result }).await
        }
        Err(e) => dispatch(&bot, chat, reply_to, user_id, &supervisor, async { Err(e) }).await,
    }
}
