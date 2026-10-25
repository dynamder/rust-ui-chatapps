use serde::{Deserialize, Serialize};

/// 消息发送方。最小单聊模型只有两个角色：本机用户与"对方"（演示中是自动回复机器人）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase")]
pub enum Sender {
    User,
    Peer,
}

impl Sender {
    pub fn as_str(self) -> &'static str {
        match self {
            Sender::User => "user",
            Sender::Peer => "peer",
        }
    }
}

/// 一个会话。时间都是 Unix 毫秒，UI 自己按本地时区格式化。
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Conversation {
    pub id: String,
    pub title: String,
    pub created_at: i64,
    pub updated_at: i64,
    /// 最后一条消息的时间；空会话为 `None`，用于列表排序。
    pub last_message_at: Option<i64>,
}

/// 会话 + 列表页要用的派生信息（最后一条消息、未读数）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSummary {
    #[serde(flatten)]
    pub conversation: Conversation,
    /// 最后一条消息，空会话为 `None`。
    pub last_message: Option<Message>,
    pub message_count: i64,
    /// 未读的 peer 消息数，用来画红色角标。
    pub unread_count: i64,
}

impl ConversationSummary {
    pub fn id(&self) -> &str {
        &self.conversation.id
    }
}

/// 一条消息。
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Message {
    /// 自增 id：全局单调递增，可直接当分页游标和排序依据。
    pub id: i64,
    pub conversation_id: String,
    pub sender: Sender,
    pub content: String,
    pub created_at: i64,
}

/// 一页历史消息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagePage {
    /// 时间升序（旧 → 新），拿到就能直接按顺序渲染。
    pub messages: Vec<Message>,
    /// 是否还有更早的消息。
    pub has_more: bool,
    /// 继续往上翻页时传给 `before_id`；没有更早的消息就是 `None`。
    pub next_before_id: Option<i64>,
}

/// 搜索结果：消息本体 + 所属会话标题，方便在结果里显示上下文。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageHit {
    #[serde(flatten)]
    pub message: Message,
    pub conversation_title: String,
}

pub(crate) fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as i64)
        .unwrap_or_default()
}
