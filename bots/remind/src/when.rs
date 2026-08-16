//! Parse "when" expressions for `/remind` into Unix timestamps.
//!
//! A reminder command is `/remind <when> <message>`. The `<when>` grammar:
//!
//! - a relative duration — `15m`, `in 2h30m`, `after 1d 12h`, `5 minutes`,
//!   `in an hour`, `next week`,
//! - a date, optionally with a clock time — `tomorrow 9am`, `monday`,
//!   `next monday`, `june 5`, `5 june`, `2025-06-01`,
//! - a clock time — `9am`, `14:30`, `noon`, `midnight`, `9 pm`,
//! - a date and time in either order, joined by an optional `at` —
//!   `9am tomorrow`, `tomorrow at 9am`, `at 9am`, `monday at 9am`.
//!
//! Absolute times are interpreted in the chat's UTC offset (see
//! `/timezone`); a moment already past rolls forward (a clock time → the
//! next day, a day of week → the next week, a month/day → the next year).
//! All parsing happens here so commands stay thin.

use anyhow::{Result, bail};
use time::{Date, Duration, Month, OffsetDateTime, PrimitiveDateTime, Time, UtcOffset, Weekday};

/// Lead times beyond this (10 years) are rejected as likely typos.
const MAX_LEAD_SECS: i64 = 10 * 365 * 86_400;

/// How a weekday is anchored to the calendar.
#[derive(Debug, Clone, Copy)]
enum WeekdayWhen {
    /// This week's occurrence (errors if that moment has passed).
    This,
    /// Next week's occurrence.
    Next,
    /// The next occurrence (rolls forward when today's moment has passed).
    Upcoming,
}

/// A date component of a "when" expression.
#[derive(Debug, Clone, Copy)]
enum DateExpr {
    Today,
    Tomorrow,
    Weekday(Weekday, WeekdayWhen),
    MonthDay { month: u8, day: u8 },
    Iso(Date),
}

/// A fully-matched "when": either a duration, or a date plus optional time.
#[derive(Debug, Clone, Copy)]
enum WhenExpr {
    Relative(i64),
    At {
        date: DateExpr,
        time: Option<(u8, u8)>,
    },
}

/// Resolves "when" expressions against a fixed wall clock (now) and a UTC
/// offset.
#[derive(Debug, Clone, Copy)]
pub struct When {
    now_secs: i64,
    offset: UtcOffset,
}

impl When {
    /// Build a resolver. `utc_offset_minutes` is east of UTC (e.g. `+330`).
    pub fn new(now_secs: i64, utc_offset_minutes: i16) -> Self {
        let offset = UtcOffset::from_whole_seconds(i32::from(utc_offset_minutes) * 60)
            .unwrap_or(UtcOffset::UTC);
        Self { now_secs, offset }
    }

    /// Split a raw command into its leading "when" and trailing "message".
    pub fn split(raw: &str) -> Result<(String, String)> {
        let tokens: Vec<&str> = raw.split_whitespace().collect();
        if tokens.is_empty() {
            bail!("Usage: /remind <when> <message> — e.g. /remind in 15m buy milk");
        }
        let when_len = Self::match_when(&tokens)
            .map(|(n, _)| n)
            .ok_or_else(Self::parse_error)?;
        let message = tokens[when_len..].join(" ");
        if message.trim().is_empty() {
            bail!("Usage: /remind <when> <message> — add something to be reminded about");
        }
        Ok((tokens[..when_len].join(" "), message))
    }

    /// Resolve a "when" expression (from [`When::split`]) to a Unix
    /// timestamp.
    pub fn resolve(&self, when: &str) -> Result<i64> {
        let tokens: Vec<&str> = when.split_whitespace().collect();
        let expr = Self::match_when(&tokens)
            .map(|(_, expr)| expr)
            .ok_or_else(Self::parse_error)?;
        let at = self.resolve_expr(&expr)?;
        self.check_future(at)
    }

    fn parse_error() -> anyhow::Error {
        anyhow::anyhow!(
            "Couldn't parse the time — try: in 15m, 2h, tomorrow 9am, monday 9am, \
             9am, or 2025-06-01 09:00"
        )
    }

    /// Reject timestamps in the past or absurdly far out.
    fn check_future(&self, at: i64) -> Result<i64> {
        if at <= self.now_secs {
            bail!("that time is in the past");
        }
        if at > self.now_secs + MAX_LEAD_SECS {
            bail!("that's too far in the future (max 10 years)");
        }
        Ok(at)
    }

    // --- matching ------------------------------------------------------------

    /// Match a maximal leading "when" prefix, returning the token count and
    /// a structured expression. `None` when the front isn't a valid "when".
    fn match_when(tokens: &[&str]) -> Option<(usize, WhenExpr)> {
        if tokens.is_empty() {
            return None;
        }
        // A relative duration is complete on its own.
        if let Some((secs, n)) = Self::relative_secs(tokens) {
            return Some((n, WhenExpr::Relative(secs)));
        }
        // An optional leading `at` (`at 9am`).
        let base = usize::from(tokens[0].eq_ignore_ascii_case("at"));

        // date [at] [time]
        if let Some((date, n)) = Self::try_date(tokens, base) {
            let mut i = base + n;
            let mut time = None;
            if Self::at(tokens, i) && Self::time_at(tokens, i + 1).is_some() {
                i += 1;
                let (h, m, nt) = Self::time_at(tokens, i)?;
                time = Some((h, m));
                i += nt;
            } else if let Some((h, m, nt)) = Self::time_at(tokens, i) {
                time = Some((h, m));
                i += nt;
            }
            return Some((i, WhenExpr::At { date, time }));
        }

        // time [at] [date]
        if let Some((h, m, nt)) = Self::time_at(tokens, base) {
            let mut i = base + nt;
            let date = if Self::at(tokens, i) && Self::try_date(tokens, i + 1).is_some() {
                i += 1;
                let (d, n) = Self::try_date(tokens, i)?;
                i += n;
                d
            } else {
                match Self::try_date(tokens, i) {
                    Some((d, n)) => {
                        i += n;
                        d
                    }
                    None => DateExpr::Today,
                }
            };
            return Some((
                i,
                WhenExpr::At {
                    date,
                    time: Some((h, m)),
                },
            ));
        }

        None
    }

    /// Match a date component starting at `tokens[i]`, returning it and how
    /// many tokens it spans.
    fn try_date(tokens: &[&str], i: usize) -> Option<(DateExpr, usize)> {
        let token = *tokens.get(i)?;
        if token.eq_ignore_ascii_case("tomorrow") {
            return Some((DateExpr::Tomorrow, 1));
        }
        if token.eq_ignore_ascii_case("today") {
            return Some((DateExpr::Today, 1));
        }
        if let Some(date) = Self::parse_iso_date(token) {
            return Some((DateExpr::Iso(date), 1));
        }
        if (token.eq_ignore_ascii_case("this") || token.eq_ignore_ascii_case("next"))
            && let Some(day) = tokens.get(i + 1).and_then(|t| Self::parse_weekday(t))
        {
            let when = if token.eq_ignore_ascii_case("next") {
                WeekdayWhen::Next
            } else {
                WeekdayWhen::This
            };
            return Some((DateExpr::Weekday(day, when), 2));
        }
        if let Some(day) = Self::parse_weekday(token) {
            return Some((DateExpr::Weekday(day, WeekdayWhen::Upcoming), 1));
        }
        // `<month> <day>` (`june 5`); a month name alone is not a date.
        if let Some(month) = Self::parse_month(token) {
            if let Some(day) = tokens.get(i + 1).and_then(|t| Self::parse_day(t)) {
                return Some((DateExpr::MonthDay { month, day }, 2));
            }
            return None;
        }
        // `<day> <month>` (`5 june`).
        if let Some(day) = Self::parse_day(token)
            && let Some(month) = tokens.get(i + 1).and_then(|t| Self::parse_month(t))
        {
            return Some((DateExpr::MonthDay { month, day }, 2));
        }
        None
    }

    /// Whether `tokens[i]` is the connective `at`.
    fn at(tokens: &[&str], i: usize) -> bool {
        tokens.get(i).is_some_and(|t| t.eq_ignore_ascii_case("at"))
    }

    fn parse_weekday(token: &str) -> Option<Weekday> {
        let t = token.to_ascii_lowercase();
        match t.as_str() {
            "monday" | "mon" => Some(Weekday::Monday),
            "tuesday" | "tue" | "tues" => Some(Weekday::Tuesday),
            "wednesday" | "wed" => Some(Weekday::Wednesday),
            "thursday" | "thu" | "thur" | "thurs" => Some(Weekday::Thursday),
            "friday" | "fri" => Some(Weekday::Friday),
            "saturday" | "sat" => Some(Weekday::Saturday),
            "sunday" | "sun" => Some(Weekday::Sunday),
            _ => None,
        }
    }

    fn parse_month(token: &str) -> Option<u8> {
        let t = token.to_ascii_lowercase();
        match t.as_str() {
            "january" | "jan" => Some(1),
            "february" | "feb" => Some(2),
            "march" | "mar" => Some(3),
            "april" | "apr" => Some(4),
            "may" => Some(5),
            "june" | "jun" => Some(6),
            "july" | "jul" => Some(7),
            "august" | "aug" => Some(8),
            "september" | "sep" | "sept" => Some(9),
            "october" | "oct" => Some(10),
            "november" | "nov" => Some(11),
            "december" | "dec" => Some(12),
            _ => None,
        }
    }

    fn parse_day(token: &str) -> Option<u8> {
        if token.is_empty() || !token.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let day: u8 = token.parse().ok()?;
        (1..=31).contains(&day).then_some(day)
    }

    // --- resolving -----------------------------------------------------------

    fn resolve_expr(&self, expr: &WhenExpr) -> Result<i64> {
        match expr {
            WhenExpr::Relative(secs) => Ok(self.now_secs + secs),
            WhenExpr::At { date, time } => {
                let (hour, minute) = time.unwrap_or((9, 0));
                let date = self.resolve_date(date, hour, minute)?;
                Ok(Self::local_timestamp(date, hour, minute, self.offset))
            }
        }
    }

    fn resolve_date(&self, date: &DateExpr, hour: u8, minute: u8) -> Result<Date> {
        let today = self.now_local().date();
        match date {
            DateExpr::Today => {
                let at = Self::local_timestamp(today, hour, minute, self.offset);
                if at <= self.now_secs {
                    Self::add_days(today, 1)
                } else {
                    Ok(today)
                }
            }
            DateExpr::Tomorrow => Self::add_days(today, 1),
            DateExpr::Weekday(day, when) => {
                let diff = Self::days_until(today.weekday(), *day);
                match when {
                    WeekdayWhen::This => Self::add_days(today, diff),
                    WeekdayWhen::Next => Self::add_days(today, diff + 7),
                    WeekdayWhen::Upcoming => {
                        let candidate = Self::add_days(today, diff)?;
                        let at = Self::local_timestamp(candidate, hour, minute, self.offset);
                        if at <= self.now_secs {
                            Self::add_days(today, diff + 7)
                        } else {
                            Ok(candidate)
                        }
                    }
                }
            }
            DateExpr::MonthDay { month, day } => self.next_month_day(*month, *day, hour, minute),
            DateExpr::Iso(date) => Ok(*date),
        }
    }

    /// The next `month/day` at `hour:minute` in the future, scanning this
    /// year then the next.
    fn next_month_day(&self, month: u8, day: u8, hour: u8, minute: u8) -> Result<Date> {
        let month = Month::try_from(month).map_err(|_| anyhow::anyhow!("invalid month"))?;
        let year = self.now_local().year();
        for year in [year, year + 1] {
            if let Ok(date) = Date::from_calendar_date(year, month, day) {
                let at = Self::local_timestamp(date, hour, minute, self.offset);
                if date >= self.now_local().date() && at > self.now_secs {
                    return Ok(date);
                }
            }
        }
        bail!("that date doesn't exist near now")
    }

    /// Whole days from `from` to the next `to` (0 when equal).
    fn days_until(from: Weekday, to: Weekday) -> i64 {
        i64::from(
            (i32::from(to.number_from_monday()) - i32::from(from.number_from_monday()))
                .rem_euclid(7),
        )
    }

    fn add_days(date: Date, days: i64) -> Result<Date> {
        date.checked_add(Duration::days(days))
            .ok_or_else(|| anyhow::anyhow!("date out of range"))
    }

    // --- relative durations -------------------------------------------------

    /// Seconds and token count of a leading duration (`15m`, `in 2h 30m`),
    /// or `None` when the front isn't a duration.
    fn relative_secs(tokens: &[&str]) -> Option<(i64, usize)> {
        // "next week" — a fixed 7 days.
        if tokens.len() >= 2
            && tokens[0].eq_ignore_ascii_case("next")
            && tokens[1].eq_ignore_ascii_case("week")
        {
            return Some((604_800, 2));
        }
        let first = *tokens.first()?;
        let start = if first.eq_ignore_ascii_case("in") || first.eq_ignore_ascii_case("after") {
            1
        } else {
            0
        };
        let mut total: i64 = 0;
        let mut i = start;
        let mut any = false;
        while i < tokens.len() {
            if let Some(secs) = Self::duration_secs(tokens[i]) {
                total = total.checked_add(secs)?;
                any = true;
                i += 1;
            } else if let (Some(num), Some(unit)) = (
                Self::parse_count(tokens[i]),
                tokens.get(i + 1).and_then(|u| Self::unit_secs(u)),
            ) {
                total = total.checked_add(num.checked_mul(unit)?)?;
                any = true;
                i += 2;
            } else {
                break;
            }
        }
        if !any {
            return None;
        }
        Some((total, i))
    }

    /// Seconds in one compact duration token (`15m`, `2h30m`, `1d12h`).
    fn duration_secs(token: &str) -> Option<i64> {
        let t = token.to_ascii_lowercase();
        if t.is_empty() {
            return None;
        }
        let bytes = t.as_bytes();
        let mut i = 0;
        let mut total: i64 = 0;
        while i < bytes.len() {
            let start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if i == start {
                return None;
            }
            let num: i64 = t[start..i].parse().ok()?;
            let unit_start = i;
            while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
                i += 1;
            }
            if i == unit_start {
                return None;
            }
            let mult = Self::unit_secs(&t[unit_start..i])?;
            total = total.checked_add(num.checked_mul(mult)?)?;
        }
        Some(total)
    }

    fn unit_secs(unit: &str) -> Option<i64> {
        let unit = unit.to_ascii_lowercase();
        match unit.as_str() {
            "s" | "sec" | "secs" | "second" | "seconds" => Some(1),
            "m" | "min" | "mins" | "minute" | "minutes" => Some(60),
            "h" | "hr" | "hrs" | "hour" | "hours" => Some(3600),
            "d" | "day" | "days" => Some(86_400),
            "w" | "wk" | "wks" | "week" | "weeks" => Some(604_800),
            _ => None,
        }
    }

    /// A count token: an integer, or `a`/`an` as one (`in an hour`).
    fn parse_count(token: &str) -> Option<i64> {
        if token.eq_ignore_ascii_case("a") || token.eq_ignore_ascii_case("an") {
            return Some(1);
        }
        if token.is_empty() || !token.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        token.parse().ok()
    }

    // --- clock times ---------------------------------------------------------

    /// Parse a clock time starting at `tokens[i]`, returning
    /// `(hour, minute, tokens consumed)`. A 12-hour clock may be split
    /// across two tokens (`"9 pm"`).
    fn time_at(tokens: &[&str], i: usize) -> Option<(u8, u8, usize)> {
        let token = *tokens.get(i)?;
        // "9 pm" / "9:30 pm" — a 12-hour clock followed by an am/pm token.
        if let Some(ampm) = tokens.get(i + 1)
            && Self::is_ampm(ampm)
            && let Some((h, m)) = Self::parse_clock(token)
        {
            return Some((Self::apply_ampm(h, ampm), m, 2));
        }
        // A single token: noon, midnight, HH:MM (24h), or H[:MM]am|pm.
        Self::parse_time_token(token).map(|(h, m)| (h, m, 1))
    }

    fn is_ampm(token: &str) -> bool {
        token.eq_ignore_ascii_case("am") || token.eq_ignore_ascii_case("pm")
    }

    fn parse_time_token(token: &str) -> Option<(u8, u8)> {
        let t = token.to_ascii_lowercase();
        match t.as_str() {
            "noon" => return Some((12, 0)),
            "midnight" => return Some((0, 0)),
            _ => {}
        }
        if let Some((h, m, ampm)) = Self::parse_attached_ampm(&t) {
            return Some((Self::apply_ampm(h, ampm), m));
        }
        Self::split_hhmm(&t).filter(|(h, m)| *h <= 23 && *m <= 59)
    }

    /// A 12-hour clock without an am/pm suffix (hour `1..=12`).
    fn parse_clock(token: &str) -> Option<(u8, u8)> {
        let (h, m) = Self::split_hhmm(token)?;
        if !(1..=12).contains(&h) || m > 59 {
            return None;
        }
        Some((h, m))
    }

    /// A clock with a trailing am/pm (`9am`, `9:30pm`).
    fn parse_attached_ampm(t: &str) -> Option<(u8, u8, &str)> {
        let (clock, ampm) = if let Some(stripped) = t.strip_suffix("am") {
            (stripped, "am")
        } else {
            let stripped = t.strip_suffix("pm")?;
            (stripped, "pm")
        };
        let (h, m) = Self::split_hhmm(clock)?;
        if !(1..=12).contains(&h) || m > 59 {
            return None;
        }
        Some((h, m, ampm))
    }

    fn apply_ampm(hour: u8, ampm: &str) -> u8 {
        match ampm.to_ascii_lowercase().as_str() {
            "pm" if hour < 12 => hour + 12,
            "am" if hour == 12 => 0,
            _ => hour,
        }
    }

    /// Split `H` or `H:MM` into `(hour, minute)`.
    fn split_hhmm(token: &str) -> Option<(u8, u8)> {
        let mut parts = token.split(':');
        let hour: u8 = parts.next()?.parse().ok()?;
        match parts.next() {
            None => Some((hour, 0)),
            Some(mm) => {
                if parts.next().is_some() {
                    return None;
                }
                let minute: u8 = mm.parse().ok()?;
                if minute > 59 {
                    return None;
                }
                Some((hour, minute))
            }
        }
    }

    fn parse_iso_date(token: &str) -> Option<Date> {
        let mut parts = token.split('-');
        let year: i32 = parts.next()?.parse().ok()?;
        let month: u8 = parts.next()?.parse().ok()?;
        let day: u8 = parts.next()?.parse().ok()?;
        if parts.next().is_some() {
            return None;
        }
        let month = Month::try_from(month).ok()?;
        Date::from_calendar_date(year, month, day).ok()
    }

    /// The current instant in this resolver's offset.
    fn now_local(&self) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(self.now_secs)
            .unwrap_or(OffsetDateTime::UNIX_EPOCH)
            .to_offset(self.offset)
    }

    /// The Unix timestamp of `hour:minute` on `date` in this offset.
    fn local_timestamp(date: Date, hour: u8, minute: u8, offset: UtcOffset) -> i64 {
        PrimitiveDateTime::new(date, Time::from_hms(hour, minute, 0).expect("valid time"))
            .assume_offset(offset)
            .unix_timestamp()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A resolver pinned to 2025-06-01 12:00:00 UTC (a Sunday).
    fn resolver(offset_minutes: i16) -> When {
        When::new(1_748_779_200, offset_minutes)
    }

    fn split(raw: &str) -> Result<(String, String)> {
        When::split(raw)
    }

    #[test]
    fn split_relative_in() -> Result<()> {
        assert_eq!(
            split("in 15m buy milk")?,
            ("in 15m".into(), "buy milk".into())
        );
        Ok(())
    }

    #[test]
    fn split_relative_bare() -> Result<()> {
        assert_eq!(split("15m buy milk")?, ("15m".into(), "buy milk".into()));
        Ok(())
    }

    #[test]
    fn split_tomorrow_with_time() -> Result<()> {
        assert_eq!(
            split("tomorrow 9am standup")?,
            ("tomorrow 9am".into(), "standup".into())
        );
        Ok(())
    }

    #[test]
    fn split_iso_date() -> Result<()> {
        assert_eq!(
            split("2025-06-05 09:00 birthday")?,
            ("2025-06-05 09:00".into(), "birthday".into())
        );
        Ok(())
    }

    #[test]
    fn split_bare_time() -> Result<()> {
        assert_eq!(split("9am standup")?, ("9am".into(), "standup".into()));
        Ok(())
    }

    #[test]
    fn split_requires_a_message() {
        assert!(split("in 15m").is_err());
        assert!(split("tomorrow 9am").is_err());
        assert!(split("").is_err());
    }

    #[test]
    fn split_rejects_garbage() {
        assert!(split("buy milk").is_err());
        assert!(split("call me tomorrow").is_err());
    }

    #[test]
    fn relative_durations() -> Result<()> {
        let w = resolver(0);
        assert_eq!(w.resolve("15m")?, w.now_secs + 900);
        assert_eq!(w.resolve("2h")?, w.now_secs + 7200);
        assert_eq!(w.resolve("in 1d")?, w.now_secs + 86_400);
        assert_eq!(w.resolve("2h30m")?, w.now_secs + 9000);
        assert_eq!(w.resolve("in 1d 12h")?, w.now_secs + 129_600);
        Ok(())
    }

    #[test]
    fn relative_word_units() -> Result<()> {
        let w = resolver(0);
        assert_eq!(w.resolve("30 seconds")?, w.now_secs + 30);
        assert_eq!(w.resolve("5 minutes")?, w.now_secs + 300);
        assert_eq!(w.resolve("2 hours")?, w.now_secs + 7200);
        assert_eq!(w.resolve("1 day")?, w.now_secs + 86_400);
        assert_eq!(w.resolve("15 M")?, w.now_secs + 900);
        Ok(())
    }

    #[test]
    fn article_and_next_week() -> Result<()> {
        let w = resolver(0);
        assert_eq!(w.resolve("in an hour")?, w.now_secs + 3600);
        assert_eq!(w.resolve("in a week")?, w.now_secs + 604_800);
        assert_eq!(w.resolve("next week")?, w.now_secs + 604_800);
        Ok(())
    }

    #[test]
    fn zero_duration_is_rejected() {
        let w = resolver(0);
        assert!(w.resolve("0m").is_err());
    }

    #[test]
    fn absurd_duration_is_rejected() {
        let w = resolver(0);
        assert!(w.resolve("999999d").is_err());
    }

    #[test]
    fn bare_clock_time_today() -> Result<()> {
        // 12:00 UTC now; 14:30 is later today.
        let w = resolver(0);
        assert_eq!(w.resolve("14:30")?, w.now_secs + 2 * 3600 + 30 * 60);
        Ok(())
    }

    #[test]
    fn bare_clock_time_rolls_to_tomorrow_when_past() -> Result<()> {
        // 09:00 today is already past (now is 12:00).
        let w = resolver(0);
        assert_eq!(w.resolve("9am")?, w.now_secs + 21 * 3600);
        Ok(())
    }

    #[test]
    fn am_pm_attached_and_split() -> Result<()> {
        let w = resolver(0);
        // 9pm today (now 12:00) is later today: +9h.
        assert_eq!(w.resolve("9pm")?, w.now_secs + 9 * 3600);
        // Split "9 pm" resolves identically.
        assert_eq!(w.resolve("9 pm")?, w.now_secs + 9 * 3600);
        Ok(())
    }

    #[test]
    fn noon_and_midnight() -> Result<()> {
        let w = resolver(0);
        // "noon" at exactly 12:00 is not in the future → rolls to tomorrow.
        assert_eq!(w.resolve("noon")?, w.now_secs + 86_400);
        // A second before noon, "noon" is later today.
        let earlier = When::new(w.now_secs - 1, 0);
        assert_eq!(earlier.resolve("noon")?, w.now_secs);
        // Midnight has passed today → rolls to tomorrow 00:00.
        assert_eq!(w.resolve("midnight")?, w.now_secs + 12 * 3600);
        Ok(())
    }

    #[test]
    fn tomorrow_defaults_to_9am() -> Result<()> {
        let w = resolver(0);
        // Tomorrow 09:00 UTC is 21h after 12:00 today.
        assert_eq!(w.resolve("tomorrow")?, w.now_secs + 21 * 3600);
        Ok(())
    }

    #[test]
    fn tomorrow_with_time() -> Result<()> {
        let w = resolver(0);
        assert_eq!(w.resolve("tomorrow 8pm")?, w.now_secs + 32 * 3600);
        Ok(())
    }

    #[test]
    fn at_connective() -> Result<()> {
        let w = resolver(0);
        assert_eq!(w.resolve("tomorrow at 9am")?, w.now_secs + 21 * 3600);
        assert_eq!(w.resolve("at 9am")?, w.now_secs + 21 * 3600);
        assert_eq!(w.resolve("monday at 9am")?, w.now_secs + 21 * 3600);
        Ok(())
    }

    #[test]
    fn time_before_date() -> Result<()> {
        let w = resolver(0);
        assert_eq!(w.resolve("9am tomorrow")?, w.now_secs + 21 * 3600);
        assert_eq!(w.resolve("9am monday")?, w.now_secs + 21 * 3600);
        assert_eq!(w.resolve("noon tomorrow")?, w.now_secs + 24 * 3600);
        Ok(())
    }

    #[test]
    fn day_of_week() -> Result<()> {
        // Today is Sunday; Monday 09:00 is 21h away.
        let w = resolver(0);
        assert_eq!(w.resolve("monday")?, w.now_secs + 21 * 3600);
        assert_eq!(w.resolve("mon")?, w.now_secs + 21 * 3600);
        assert_eq!(w.resolve("this monday")?, w.now_secs + 21 * 3600);
        // "next monday" is a week later than "this monday".
        assert_eq!(w.resolve("next monday")?, w.now_secs + 189 * 3600);
        Ok(())
    }

    #[test]
    fn this_weekday_past_is_rejected() {
        // Monday 15:00 local — "this monday" (09:00) has already passed.
        let monday = resolver(0).now_secs + 27 * 3600;
        let w = When::new(monday, 0);
        assert!(w.resolve("this monday").is_err());
    }

    #[test]
    fn month_day() -> Result<()> {
        // June 5 09:00 is 93h after June 1 12:00.
        let w = resolver(0);
        assert_eq!(w.resolve("june 5")?, w.now_secs + 93 * 3600);
        assert_eq!(w.resolve("5 june")?, w.now_secs + 93 * 3600);
        assert_eq!(w.resolve("jun 5 9am")?, w.now_secs + 93 * 3600);
        assert_eq!(w.resolve("9am june 5")?, w.now_secs + 93 * 3600);
        Ok(())
    }

    #[test]
    fn iso_date_resolves() -> Result<()> {
        let w = resolver(0);
        // 2025-06-02 09:00 UTC = 21h after 2025-06-01 12:00.
        assert_eq!(w.resolve("2025-06-02 09:00")?, w.now_secs + 21 * 3600);
        Ok(())
    }

    #[test]
    fn iso_date_in_the_past_is_rejected() {
        let w = resolver(0);
        assert!(w.resolve("2025-05-01").is_err());
    }

    #[test]
    fn offset_shifts_clock_times() -> Result<()> {
        // In UTC+2, 09:00 local is 07:00 UTC — but it's already past
        // (12:00 UTC now), so it rolls to tomorrow 07:00 UTC.
        let w = resolver(120);
        assert_eq!(w.resolve("9am")?, w.now_secs + 19 * 3600);
        Ok(())
    }

    #[test]
    fn duration_parser_rejects_non_durations() {
        assert_eq!(When::duration_secs("buy"), None);
        assert_eq!(When::duration_secs("9am"), None);
        assert_eq!(When::duration_secs("2025-06-01"), None);
        assert_eq!(When::duration_secs(""), None);
    }

    #[test]
    fn split_hhmm_variants() {
        assert_eq!(When::split_hhmm("9"), Some((9, 0)));
        assert_eq!(When::split_hhmm("09:00"), Some((9, 0)));
        assert_eq!(When::split_hhmm("14:30"), Some((14, 30)));
        assert_eq!(When::split_hhmm("24:00"), Some((24, 0)));
        assert_eq!(When::split_hhmm("9:60"), None);
        assert_eq!(When::split_hhmm("9:30:00"), None);
        assert_eq!(When::split_hhmm("x"), None);
    }
}
