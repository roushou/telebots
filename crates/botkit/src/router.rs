//! The update router: composes command and inline-query branches into a
//! single dispatcher tree sharing one context.

use teloxide::{
    Bot as Api, RequestError,
    dispatching::{UpdateFilterExt as _, UpdateHandler},
    prelude::Requester,
    requests::ResponseResult,
    types::{CallbackQuery, ChatId, InlineQuery, Me, Message, MessageId, Update},
};

use crate::{
    callback::{CallbackHandler, CallbackRequest},
    command::{Command, MenuEntry},
    dispatch::{MAX_MESSAGE_LEN, Supervisor, dispatch},
    guard::{Guard, NoGuard},
    inline::{InlineHandler, InlineRequest},
    messenger::Messenger,
    reply::Reply,
    request::Request,
};

/// A set of update handlers sharing one context, assembled before
/// [`crate::Bot::run`].
///
/// A branch builds its slice of the dispatcher tree from the shared
/// context type.
type Branch = Box<dyn FnOnce() -> UpdateHandler<RequestError> + Send + Sync>;

pub struct Router<Ctx> {
    ctx: Ctx,
    menu: Vec<MenuEntry>,
    branches: Vec<Branch>,
}

impl<Ctx: Clone + Send + Sync + 'static> Router<Ctx> {
    /// A router with its shared command context.
    pub fn new(ctx: Ctx) -> Self {
        Self {
            ctx,
            menu: Vec::new(),
            branches: Vec::new(),
        }
    }

    /// Handle `/command` messages with the given command enum.
    pub fn command<C>(self) -> Self
    where
        C: Command<Ctx = Ctx>,
    {
        self.guarded_command::<C, NoGuard>(NoGuard)
    }

    /// Handle `/command` messages, running `guard` before each command.
    pub fn guarded_command<C, G>(mut self, guard: G) -> Self
    where
        C: Command<Ctx = Ctx>,
        G: Guard<C, Ctx>,
    {
        self.menu.extend(C::menu());
        self.branches
            .push(Box::new(move || command_branch::<C, G>(guard)));
        self
    }

    /// Handle `@botname <query>` inline queries with the given handler.
    pub fn inline_query<I>(mut self, handler: I) -> Self
    where
        I: InlineHandler<Ctx = Ctx>,
    {
        self.branches.push(Box::new(move || inline_branch(handler)));
        self
    }

    /// Handle inline-keyboard button taps.
    pub fn callback<H>(mut self, handler: H) -> Self
    where
        H: CallbackHandler<Ctx = Ctx>,
    {
        self.branches
            .push(Box::new(move || callback_branch(handler)));
        self
    }

    /// Add the built-in `/stats` admin command.
    pub fn stats(mut self) -> Self {
        self.menu.push(MenuEntry {
            command: "/stats".into(),
            description: "Show bot stats".into(),
        });
        self.branches.push(Box::new(crate::stats::stats_branch));
        self
    }

    pub(crate) fn into_parts(self) -> (Ctx, Vec<MenuEntry>, Vec<Branch>) {
        (self.ctx, self.menu, self.branches)
    }
}

/// The command branch: parse `/command` text, then run the guard and the
/// command.
fn command_branch<C, G>(guard: G) -> UpdateHandler<RequestError>
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
fn inline_branch<I>(handler: I) -> UpdateHandler<RequestError>
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
                    .map(crate::inline::InlineResult::into_telegram)
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
fn callback_branch<H>(handler: H) -> UpdateHandler<RequestError>
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
