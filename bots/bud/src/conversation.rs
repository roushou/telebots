//! Conversation assembly: turn stored history into the ordered message list
//! the LLM sees.

use cloudflare_ai::{ChatMessage, Role};

use crate::store::StoredMessage;

/// Builds the ordered chat messages for a completion request.
pub struct Conversation;

impl Conversation {
    /// Assemble `system_prompt` + `history` (oldest first, already ending
    /// with the latest user message) into provider messages.
    pub fn build(system_prompt: &str, history: &[StoredMessage]) -> Vec<ChatMessage> {
        let mut out = Vec::with_capacity(history.len() + 1);
        out.push(ChatMessage::system(system_prompt));
        for message in history {
            let role = match message.role.as_str() {
                "user" => Role::User,
                "assistant" => Role::Assistant,
                // Skip anything we don't recognize rather than corrupting
                // the conversation.
                _ => continue,
            };
            out.push(ChatMessage::new(role, message.content.clone()));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(role: &str, content: &str) -> StoredMessage {
        StoredMessage {
            role: role.to_string(),
            content: content.to_string(),
        }
    }

    #[test]
    fn builds_system_then_history() {
        let history = [msg("user", "hi"), msg("assistant", "hello")];
        let out = Conversation::build("be nice", &history);
        assert_eq!(out.len(), 3);
        assert_eq!(out[0].role, Role::System);
        assert_eq!(out[0].content, "be nice");
        assert_eq!(out[1].role, Role::User);
        assert_eq!(out[2].role, Role::Assistant);
    }

    #[test]
    fn skips_unknown_roles() {
        let history = [msg("user", "hi"), msg("system", "nope")];
        let out = Conversation::build("be nice", &history);
        assert_eq!(out.len(), 2);
        assert_eq!(out[1].role, Role::User);
    }
}
