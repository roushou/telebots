//! Inline keyboards: rows of callback buttons attached to replies.

/// An inline keyboard: rows of buttons.
#[derive(Debug, Clone, Default)]
pub struct Markup {
    rows: Vec<Vec<Button>>,
}

/// One button in an inline keyboard.
#[derive(Debug, Clone)]
pub struct Button {
    text: String,
    data: String,
}

impl Markup {
    /// An empty keyboard.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a row of buttons.
    pub fn row(mut self, buttons: impl IntoIterator<Item = Button>) -> Self {
        self.rows.push(buttons.into_iter().collect());
        self
    }

    /// Whether there are no buttons.
    pub fn is_empty(&self) -> bool {
        self.rows.iter().all(Vec::is_empty)
    }

    pub(crate) fn into_telegram(self) -> teloxide::types::InlineKeyboardMarkup {
        teloxide::types::InlineKeyboardMarkup {
            inline_keyboard: self
                .rows
                .into_iter()
                .map(|row| row.into_iter().map(Button::into_telegram).collect())
                .collect(),
        }
    }
}

impl Button {
    /// A button that sends `data` back to the bot when tapped.
    pub fn callback(text: impl Into<String>, data: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            data: data.into(),
        }
    }

    fn into_telegram(self) -> teloxide::types::InlineKeyboardButton {
        teloxide::types::InlineKeyboardButton {
            text: self.text,
            kind: teloxide::types::InlineKeyboardButtonKind::CallbackData(self.data),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_to_telegram_keyboard() {
        let markup = Markup::new()
            .row([Button::callback("Refresh", "refresh")])
            .row([Button::callback("A", "a"), Button::callback("B", "b")]);

        let keyboard = markup.into_telegram();
        assert_eq!(keyboard.inline_keyboard.len(), 2);
        assert_eq!(keyboard.inline_keyboard[0][0].text, "Refresh");
        assert_eq!(keyboard.inline_keyboard[1].len(), 2);
        match &keyboard.inline_keyboard[1][1].kind {
            teloxide::types::InlineKeyboardButtonKind::CallbackData(data) => {
                assert_eq!(data, "b")
            }
            other => panic!("expected callback data, got {other:?}"),
        }
    }
}
