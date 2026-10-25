use serde::Serialize;

use crate::model::{Conversation, Message, Sender};

/// 数据变更事件。UI 订阅后按 `type` 分发即可。
///
/// 序列化后形如 `{"type":"message_created","message":{...}}`。
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatEvent {
    ConversationCreated {
        conversation: Conversation,
    },
    /// 标题变了 / 有消息进出导致 `updated_at` 变化。
    ConversationUpdated {
        conversation: Conversation,
    },
    ConversationDeleted {
        conversation_id: String,
    },
    /// 会话里的消息被清空（`last_message_at` 也一起重置）。
    MessagesCleared {
        conversation_id: String,
    },
    /// 已读水位推进，`unread_count` 应变成 0。
    ConversationRead {
        conversation_id: String,
        last_read_message_id: Option<i64>,
    },
    MessageCreated {
        message: Message,
    },
    /// 对方"正在输入"。延迟回复的机器人会先发 `is_typing: true`。
    Typing {
        conversation_id: String,
        sender: Sender,
        is_typing: bool,
    },
}

impl ChatEvent {
    /// 事件所属会话，SSE 按会话过滤时用。
    pub fn conversation_id(&self) -> Option<&str> {
        match self {
            ChatEvent::ConversationCreated { conversation }
            | ChatEvent::ConversationUpdated { conversation } => Some(&conversation.id),
            ChatEvent::ConversationDeleted { conversation_id }
            | ChatEvent::MessagesCleared { conversation_id }
            | ChatEvent::ConversationRead {
                conversation_id, ..
            }
            | ChatEvent::Typing {
                conversation_id, ..
            } => Some(conversation_id),
            ChatEvent::MessageCreated { message } => Some(&message.conversation_id),
        }
    }
}
