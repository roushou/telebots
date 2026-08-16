//! `/timezone` — set the chat's UTC offset for clock-time reminders.

use anyhow::{Result, bail};
use botkit::Reply;

use crate::{commands::Ctx, render};

/// Typed arguments for `/timezone`.
pub struct TimezoneArgs {
    minutes: i16,
}

impl TimezoneArgs {
    /// Parse a UTC offset like `+2`, `-5`, `+5:30`, or `utc`.
    pub fn parse(raw: &str) -> Result<Self> {
        let t = raw.trim();
        if t.is_empty() {
            bail!("Usage: /timezone <offset> — e.g. +2, -5, +5:30, or utc");
        }
        Ok(Self {
            minutes: Self::parse_offset(t)?,
        })
    }

    fn parse_offset(raw: &str) -> Result<i16> {
        let mut t = raw.to_ascii_lowercase();
        for prefix in ["utc", "gmt"] {
            if t == prefix {
                return Ok(0);
            }
            if let Some(stripped) = t.strip_prefix(prefix) {
                t = stripped.to_string();
                break;
            }
        }
        let (sign, rest) = match t.as_bytes().first() {
            Some(b'-') => (-1, &t[1..]),
            Some(b'+') => (1, &t[1..]),
            _ => (1, t.as_str()),
        };
        let mut parts = rest.split(':');
        let hours: i16 = parts.next().and_then(|s| s.parse().ok()).ok_or_else(|| {
            anyhow::anyhow!("Usage: /timezone <offset> — e.g. +2, -5, +5:30, or utc")
        })?;
        let minutes: i16 = match parts.next() {
            None => 0,
            Some(m) => {
                if parts.next().is_some() {
                    bail!("Usage: /timezone <offset> — e.g. +2, -5, +5:30, or utc");
                }
                m.parse().map_err(|_| {
                    anyhow::anyhow!("Usage: /timezone <offset> — e.g. +2, -5, +5:30, or utc")
                })?
            }
        };
        if minutes > 59 {
            bail!("minutes must be 0–59");
        }
        let total = sign * (hours * 60 + minutes);
        if !(-720..=840).contains(&total) {
            bail!("offset must be between -12:00 and +14:00");
        }
        Ok(total)
    }

    /// Produce the reply: persist the offset and confirm.
    pub async fn reply(&self, ctx: &Ctx, chat_id: i64) -> Result<Reply> {
        ctx.store.set_utc_offset(chat_id, self.minutes).await?;
        Ok(Reply::text(render::timezone_set(self.minutes)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_accepts_offsets() -> Result<()> {
        assert_eq!(TimezoneArgs::parse("utc")?.minutes, 0);
        assert_eq!(TimezoneArgs::parse("GMT")?.minutes, 0);
        assert_eq!(TimezoneArgs::parse("+2")?.minutes, 120);
        assert_eq!(TimezoneArgs::parse("-5")?.minutes, -300);
        assert_eq!(TimezoneArgs::parse("+5:30")?.minutes, 330);
        assert_eq!(TimezoneArgs::parse("-3:30")?.minutes, -210);
        assert_eq!(TimezoneArgs::parse("utc+2")?.minutes, 120);
        assert_eq!(TimezoneArgs::parse("2")?.minutes, 120);
        Ok(())
    }

    #[test]
    fn parse_rejects_garbage() {
        assert!(TimezoneArgs::parse("").is_err());
        assert!(TimezoneArgs::parse("+abc").is_err());
        assert!(TimezoneArgs::parse("+5:60").is_err());
        assert!(TimezoneArgs::parse("+20").is_err());
        assert!(TimezoneArgs::parse("-15").is_err());
        assert!(TimezoneArgs::parse("+5:30:00").is_err());
    }
}
