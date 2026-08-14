//! The update router: composes command and inline-query branches into a
//! single dispatcher tree sharing one context.

use teloxide::{
    Bot as Api, RequestError,
    dispatching::{UpdateFilterExt as _, UpdateHandler},
    prelude::Requester,
    requests::ResponseResult,
    types::{InlineQuery, Me, Message, Update},
};

use crate::{
    command::{Command, MenuEntry},
    dispatch::{Supervisor, dispatch},
    guard::{Guard, NoGuard},
    inline::{InlineHandler, InlineRequest},
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
    match guard.check(&ctx, &req, &cmd).await {
        Ok(Some(reply)) => dispatch(&bot, &msg, &supervisor, async { Ok(reply) }).await,
        Ok(None) => dispatch(&bot, &msg, &supervisor, cmd.reply(&ctx, &req)).await,
        Err(e) => dispatch(&bot, &msg, &supervisor, async { Err(e) }).await,
    }
}
