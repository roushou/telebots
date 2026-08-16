//! The update router: composes command, inline-query, callback, message, and
//! built-in branches into a single dispatcher tree sharing one context.

use telebots_core::Block;
use teloxide::{RequestError, dispatching::UpdateHandler};

use crate::{
    branches::{callback_branch, command_branch, inline_branch, message_branch},
    builtin::{help_branch, stats_branch},
    guard::{Guard, NoGuard},
    handlers::{CallbackHandler, Command, CommandSpec, InlineHandler, MenuEntry, MessageHandler},
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

    /// Handle free-form text messages not consumed by a command branch.
    ///
    /// Register this **after** `.command()` so `/command` messages are
    /// parsed first and never leak into the message handler. The handler
    /// may return `None` to stay silent.
    pub fn message<H>(mut self, handler: H) -> Self
    where
        H: MessageHandler<Ctx = Ctx>,
    {
        self.branches
            .push(Box::new(move || message_branch(handler)));
        self
    }

    /// Add the built-in `/stats` admin command.
    pub fn stats(mut self) -> Self {
        self.menu.push(MenuEntry {
            command: "/stats".into(),
            description: "Show bot stats".into(),
        });
        self.branches.push(Box::new(stats_branch));
        self
    }

    /// Add the built-in `/help` command, rendering `C`'s command list plus
    /// an optional bot-specific tail block (models, notes, ...).
    pub fn help<C>(mut self, extra: Option<Block>) -> Self
    where
        C: CommandSpec,
    {
        self.menu.push(MenuEntry {
            command: "/help".into(),
            description: "Show help".into(),
        });
        self.branches
            .push(Box::new(move || help_branch::<C>(extra)));
        self
    }

    pub(crate) fn into_parts(self) -> (Ctx, Vec<MenuEntry>, Vec<Branch>) {
        (self.ctx, self.menu, self.branches)
    }
}
