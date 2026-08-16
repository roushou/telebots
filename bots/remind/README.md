# Remind

A Telegram reminders bot: set a reminder with a natural-ish time expression
(`/remind in 15m buy milk`), and get pinged when it's due. Reminders persist
in SQLite, so they survive restarts; a scheduler delivers them (catching up
on anything that came due while the bot was offline).

## Setup

```sh
cp bots/remind/.env.example bots/remind/.env   # fill in the keys
mise run bot remind                            # cargo run -p remind
```

Use a separate test bot token from @BotFather for local dev.

| Variable                    | Where to get it                                             |
| --------------------------- | ----------------------------------------------------------- |
| `TELEBOTS_TELEGRAM_API_KEY` | Telegram bot token from [@BotFather]                        |
| `REMIND_DB_PATH`            | SQLite path (default `remind.db` locally; `/data/remind.db` in the container, set in docker-compose.yml) |
| `TELEBOTS_METRICS_PORT`     | Optional: metrics port (default `9104`)                     |

[@BotFather]: https://t.me/BotFather

## Commands

| Command                     | Description                          |
| --------------------------- | ------------------------------------ |
| `/remind <when> <message>`  | Set a reminder                       |
| `/reminders`                | List upcoming reminders              |
| `/cancel <number>`          | Remove a reminder by its list number |
| `/timezone <offset>`        | Set your UTC offset (e.g. `+2`, `-5`, `+5:30`, `utc`) |
| `/stats`                    | Bot uptime, commands, jobs, panics   |
| `/help`                     | Show this help                       |

## When to remind

`<when>` is one of:

| Form               | Meaning                                   |
| ------------------ | ----------------------------------------- |
| `15m`, `2h30m`     | relative duration (`in`/`after` optional) |
| `5 minutes`, `1d`  | word units (`s`, `m`, `h`, `d`, `w`)      |
| `next week`, `in an hour` | a week from now / one hour           |
| `9am`, `14:30`, `noon`, `midnight` | a clock time today (or tomorrow if already past) |
| `monday`, `mon`    | next occurrence of a weekday              |
| `next monday`, `this monday` | next week's / this week's Monday  |
| `june 5`, `5 june` | next occurrence of a month/day            |
| `tomorrow 9am`, `9am tomorrow` | a date and time in either order |
| `2025-06-01 09:00` | an exact date (and optional time)         |

A time can be joined to a date with `at`: `tomorrow at 9am`, `monday at 9am`,
`at 9am`.

Examples:

```text
/remind in 15m buy milk
/remind 2h30m stretch break
/remind monday 9am standup
/remind june 5 mum's birthday
/remind 9am tomorrow call the dentist
```

## Notes

- Clock times are interpreted in the chat's UTC offset (default UTC); set it
  with `/timezone +2`. Offsets are fixed (no DST awareness) — re-set twice a
  year if you observe DST.
- Delivery is at-least-once: reminders are deleted only after Telegram
  acknowledges the message, so a crash between send and acknowledge can
  redeliver.
- The scheduler checks every 15 seconds and runs its first tick immediately,
  so reminders that came due while the bot was offline fire on startup.
- The time parser lives in `src/when.rs` (pure, heavily tested); the store in
  `src/store.rs`; the scheduler glue in `src/scheduler.rs` feeding botkit's
  [`ScheduleSource`](../../crates/botkit/src/scheduler.rs).
