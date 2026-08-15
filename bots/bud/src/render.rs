//! Presentation: bud's data rendered into `Block`s.

use cloudflare_ai::TextModel;
use telebots_core::{Block, Cell, Line};

use crate::store::StoredMessage;

/// Cap each history line so long replies stay readable.
const LINE_LIMIT: usize = 200;

/// The assistant's reply as a block.
pub fn answer(text: &str) -> Block {
    let mut b = Block::new();
    b.line(text);
    b
}

/// A conversation-history listing, one line per message.
pub fn history_block(messages: &[StoredMessage]) -> Block {
    let mut b = Block::new();
    if messages.is_empty() {
        b.line("No messages yet — just say hello 👋");
        return b;
    }
    b.line(format!("💬 Your recent conversation ({}):", messages.len()));
    for message in messages {
        let who = match message.role.as_str() {
            "user" => "you",
            "assistant" => "bud",
            other => other,
        };
        let mut line = Line::text(format!("{who}: {}", message.content));
        line.ellipsize(LINE_LIMIT);
        b.push(line);
    }
    b
}

/// The model list table shown in `/help`.
pub fn model_table() -> Block {
    let mut b = Block::new();
    b.line("Models — set with /model <name> (default llama-3.1-8b):");
    for model in TextModel::ALL {
        b.row([Cell::new(model.as_str()), Cell::new(model.description())]);
    }
    b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answer_is_a_single_line() {
        assert_eq!(answer("hello").build(), "hello");
    }

    #[test]
    fn history_renders_speaker_labels() {
        let messages = [
            StoredMessage {
                role: "user".into(),
                content: "hi".into(),
            },
            StoredMessage {
                role: "assistant".into(),
                content: "hey".into(),
            },
        ];
        let out = history_block(&messages).build();
        assert!(out.contains("you: hi"));
        assert!(out.contains("bud: hey"));
    }

    #[test]
    fn history_empty_has_hint() {
        assert!(history_block(&[]).build().contains("No messages yet"));
    }

    #[test]
    fn model_table_lists_default() {
        let out = model_table().build();
        assert!(out.contains("llama-3.1-8b"));
        assert!(out.contains("deepseek-r1"));
    }
}
