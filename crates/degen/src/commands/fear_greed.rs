//! `/fear_greed` — the Fear & Greed index (CMC keyless public API).

use anyhow::Result;
use teloxide::prelude::*;

use crate::{cmc::CmcClient, commands::util};

pub async fn handle(bot: Bot, msg: Message, cmc: CmcClient) -> ResponseResult<()> {
    util::send(bot, msg, text(&cmc).await).await
}

/// Pure command logic; unit-testable without a bot or network.
pub async fn text(cmc: &CmcClient) -> Result<String> {
    Ok(format(&cmc.fear_greed().await?))
}

pub fn format(fg: &crate::cmc::FearGreed) -> String {
    format!("😱 Fear & Greed: {}/100 — {}", fg.value, fg.classification)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmc::FearGreed;

    #[test]
    fn format_index() {
        let fg = FearGreed {
            value: 29,
            classification: "Fear".into(),
        };
        assert_eq!(format(&fg), "😱 Fear & Greed: 29/100 — Fear");
    }
}
