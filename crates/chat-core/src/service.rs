use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

use sqlx::sqlite::Sqlite;
use sqlx::{QueryBuilder, SqlitePool};
use tokio::sync::broadcast;
use tracing::{debug, warn};

use crate::config::ChatConfig;
use crate::db;
use crate::error::{Error, Result};
use crate::event::ChatEvent;
use crate::model::{
    Conversation, ConversationSummary, Message, MessageHit, MessagePage, Sender, now_ms,
};

/// 会话列表 / 详情的公共 SELECT：会话本体 + 消息数 + 未读数。
///
/// 未读数的定义：`id` 大于已读水位的 peer 消息条数。用户自己发的消息不算未读。
const SUMMARY_SELECT: &str = r"
SELECT c.id,
       c.title,
       c.created_at,
       c.updated_at,
       c.last_message_at,
       (SELECT COUNT(*) FROM messages m WHERE m.conversation_id = c.id) AS message_count,
       (SELECT COUNT(*) FROM messages m
          WHERE m.conversation_id = c.id
            AND m.sender = 'peer'
            AND m.id > COALESCE(c.last_read_message_id, 0)) AS unread_count
FROM conversations c
";

/// `SUMMARY_SELECT` 的平面行，之后再拼上 `last_message`。
#[derive(Debug, sqlx::FromRow)]
struct SummaryRow {
    id: String,
    title: String,
    created_at: i64,
    updated_at: i64,
    last_message_at: Option<i64>,
    message_count: i64,
    unread_count: i64,
}

impl SummaryRow {
    fn into_summary(self, last_message: Option<Message>) -> ConversationSummary {
        ConversationSummary {
            conversation: Conversation {
                id: self.id,
                title: self.title,
                created_at: self.created_at,
                updated_at: self.updated_at,
                last_message_at: self.last_message_at,
            },
            last_message,
            message_count: self.message_count,
            unread_count: self.unread_count,
        }
    }
}

/// 搜索结果行：不能直接在 `MessageHit` 上 derive `FromRow`（有 `flatten`），
/// 所以先映射成平面结构再转换。
#[derive(Debug, sqlx::FromRow)]
struct SearchRow {
    id: i64,
    conversation_id: String,
    sender: Sender,
    content: String,
    created_at: i64,
    conversation_title: String,
}

impl From<SearchRow> for MessageHit {
    fn from(row: SearchRow) -> Self {
        MessageHit {
            message: Message {
                id: row.id,
                conversation_id: row.conversation_id,
                sender: row.sender,
                content: row.content,
                created_at: row.created_at,
            },
            conversation_title: row.conversation_title,
        }
    }
}

/// 聊天后端。`Clone` 很便宜（内部全是 `Arc` / 连接池句柄），
/// 可以随便塞进 UI 框架的状态容器。
///
/// 所有写操作都会通过 [`ChatService::subscribe`] 广播 [`ChatEvent`]；
/// 读接口则是直接查库，UI 想轮询也完全没问题。
#[derive(Clone)]
pub struct ChatService {
    pool: SqlitePool,
    config: Arc<ChatConfig>,
    events: broadcast::Sender<ChatEvent>,
    /// 每个会话的"回复代次"：新消息会 +1，过期的延迟回复据此自行放弃。
    reply_epochs: Arc<Mutex<HashMap<String, u64>>>,
}

impl ChatService {
    // ---------------------------------------------------------------- 构造

    /// 用默认配置打开磁盘数据库（不存在则创建）。
    pub async fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::open_with_config(path, ChatConfig::default()).await
    }

    pub async fn open_with_config(path: impl AsRef<Path>, config: ChatConfig) -> Result<Self> {
        let pool = db::connect_file(path.as_ref()).await?;
        Ok(Self::from_pool(pool, config))
    }

    /// 内存库，测试用。
    pub async fn open_memory() -> Result<Self> {
        Self::open_memory_with_config(ChatConfig::default()).await
    }

    pub async fn open_memory_with_config(config: ChatConfig) -> Result<Self> {
        let pool = db::connect_memory().await?;
        Ok(Self::from_pool(pool, config))
    }

    fn from_pool(pool: SqlitePool, config: ChatConfig) -> Self {
        let (events, _) = broadcast::channel(1024);
        Self {
            pool,
            config: Arc::new(config),
            events,
            reply_epochs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    // ---------------------------------------------------------------- 基础设施

    /// 想自己写 SQL 或跑统计时用。
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub fn config(&self) -> &ChatConfig {
        &self.config
    }

    /// 订阅数据变更事件。多个订阅者互不影响。
    ///
    /// 缓冲区 1024 条；UI 跟不上时 `recv` 会返回 `Lagged`，此时重新拉一次
    /// `list_conversations` / `messages` 即可（事件只是"通知"，不是数据源）。
    pub fn subscribe(&self) -> broadcast::Receiver<ChatEvent> {
        self.events.subscribe()
    }

    /// 给 SSE 之类的转发层用。
    pub fn events_sender(&self) -> broadcast::Sender<ChatEvent> {
        self.events.clone()
    }

    fn emit(&self, event: ChatEvent) {
        // 没有订阅者时 send 会失败，这完全正常。
        let _ = self.events.send(event);
    }

    // ---------------------------------------------------------------- 会话

    /// 新建会话。`title` 为空或 `None` 时用"新会话"。
    pub async fn create_conversation(&self, title: Option<&str>) -> Result<Conversation> {
        let title = self.default_title(title)?;
        let now = now_ms();
        let conversation = Conversation {
            id: uuid::Uuid::new_v4().to_string(),
            title,
            created_at: now,
            updated_at: now,
            last_message_at: None,
        };

        sqlx::query(
            "INSERT INTO conversations (id, title, created_at, updated_at, last_message_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&conversation.id)
        .bind(&conversation.title)
        .bind(conversation.created_at)
        .bind(conversation.updated_at)
        .bind(conversation.last_message_at)
        .execute(&self.pool)
        .await?;

        self.emit(ChatEvent::ConversationCreated {
            conversation: conversation.clone(),
        });
        Ok(conversation)
    }

    /// 会话列表：按最后活跃时间倒序，带最后一条消息与未读数。
    pub async fn list_conversations(&self) -> Result<Vec<ConversationSummary>> {
        let sql = format!(
            "{SUMMARY_SELECT} ORDER BY COALESCE(c.last_message_at, c.created_at) DESC, c.created_at DESC"
        );
        let rows: Vec<SummaryRow> = sqlx::query_as(&sql).fetch_all(&self.pool).await?;

        // 一次查出每个会话的最后一条消息，避免 N+1。
        let latest = self.latest_messages().await?;
        Ok(rows
            .into_iter()
            .map(|row| {
                let last = latest.get(&row.id).cloned();
                row.into_summary(last)
            })
            .collect())
    }

    pub async fn get_conversation(&self, id: &str) -> Result<ConversationSummary> {
        let sql = format!("{SUMMARY_SELECT} WHERE c.id = ?");
        let row: Option<SummaryRow> = sqlx::query_as(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        let row = row.ok_or_else(|| Error::ConversationNotFound(id.to_owned()))?;

        let last: Option<Message> = sqlx::query_as(
            "SELECT id, conversation_id, sender, content, created_at
             FROM messages WHERE conversation_id = ? ORDER BY id DESC LIMIT 1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.into_summary(last))
    }

    pub async fn rename_conversation(&self, id: &str, title: &str) -> Result<Conversation> {
        let title = self.required_title(title)?;
        let now = now_ms();
        let result = sqlx::query("UPDATE conversations SET title = ?, updated_at = ? WHERE id = ?")
            .bind(&title)
            .bind(now)
            .bind(id)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(Error::ConversationNotFound(id.to_owned()));
        }

        let conversation = self.fetch_conversation(id).await?;
        self.emit(ChatEvent::ConversationUpdated {
            conversation: conversation.clone(),
        });
        Ok(conversation)
    }

    /// 硬删除会话，消息随外键级联删除。
    pub async fn delete_conversation(&self, id: &str) -> Result<()> {
        let result = sqlx::query("DELETE FROM conversations WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(Error::ConversationNotFound(id.to_owned()));
        }

        // 让还在 sleep 的自动回复发现自己被取消。
        self.epochs().remove(id);
        self.emit(ChatEvent::ConversationDeleted {
            conversation_id: id.to_owned(),
        });
        Ok(())
    }

    // ---------------------------------------------------------------- 消息

    /// 用户发一条消息，返回已落库的消息；自动回复在后台异步产生。
    pub async fn send_user_message(&self, conversation_id: &str, content: &str) -> Result<Message> {
        let content = self.required_content(content)?;
        self.ensure_exists(conversation_id).await?;

        let message = self
            .insert_message(conversation_id, Sender::User, &content)
            .await?;
        self.spawn_auto_reply(conversation_id.to_owned());
        Ok(message)
    }

    /// 直接写一条"对方"的消息，不经过机器人（测试、导入历史、模拟推送时用）。
    pub async fn send_peer_message(&self, conversation_id: &str, content: &str) -> Result<Message> {
        let content = self.required_content(content)?;
        self.ensure_exists(conversation_id).await?;
        self.insert_message(conversation_id, Sender::Peer, &content)
            .await
    }

    /// 一页历史消息。`before_id` 传上一页的 `next_before_id` 即可继续往前翻。
    pub async fn messages(
        &self,
        conversation_id: &str,
        before_id: Option<i64>,
        limit: Option<i64>,
    ) -> Result<MessagePage> {
        self.ensure_exists(conversation_id).await?;

        let limit = limit
            .unwrap_or(self.config.default_page_size)
            .clamp(1, self.config.max_page_size);

        let mut query = QueryBuilder::<Sqlite>::new(
            "SELECT id, conversation_id, sender, content, created_at FROM messages WHERE conversation_id = ",
        );
        query.push_bind(conversation_id);
        if let Some(before) = before_id {
            query.push(" AND id < ").push_bind(before);
        }
        // 多取一条用来判断 has_more。
        query.push(" ORDER BY id DESC LIMIT ").push_bind(limit + 1);

        let mut messages: Vec<Message> = query.build_query_as().fetch_all(&self.pool).await?;
        let has_more = messages.len() as i64 > limit;
        messages.truncate(limit as usize);

        let next_before_id = if has_more {
            messages.last().map(|message| message.id)
        } else {
            None
        };
        // 查询是倒序的，返回给 UI 时统一成时间升序。
        messages.reverse();

        Ok(MessagePage {
            messages,
            has_more,
            next_before_id,
        })
    }

    /// 子串搜索（LIKE），可选限定会话，结果按 id 倒序。
    ///
    /// 说明见 README：中文场景下 LIKE 比 FTS5 的默认分词更符合直觉。
    pub async fn search_messages(
        &self,
        query: &str,
        conversation_id: Option<&str>,
        limit: Option<i64>,
    ) -> Result<Vec<MessageHit>> {
        let needle = query.trim();
        if needle.is_empty() {
            return Ok(Vec::new());
        }
        if let Some(id) = conversation_id {
            self.ensure_exists(id).await?;
        }

        let limit = limit
            .unwrap_or(self.config.max_search_results)
            .clamp(1, self.config.max_search_results);
        let pattern = format!("%{}%", escape_like(needle));

        let mut builder = QueryBuilder::<Sqlite>::new(
            "SELECT m.id, m.conversation_id, m.sender, m.content, m.created_at,
                    c.title AS conversation_title
             FROM messages m
             JOIN conversations c ON c.id = m.conversation_id
             WHERE m.content LIKE ",
        );
        builder.push_bind(pattern).push(" ESCAPE '\\'");
        if let Some(id) = conversation_id {
            builder.push(" AND m.conversation_id = ").push_bind(id);
        }
        builder.push(" ORDER BY m.id DESC LIMIT ").push_bind(limit);

        let rows: Vec<SearchRow> = builder.build_query_as().fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(MessageHit::from).collect())
    }

    /// 清空某个会话的聊天记录，返回删掉的消息条数。会话本身保留。
    pub async fn clear_messages(&self, conversation_id: &str) -> Result<u64> {
        self.ensure_exists(conversation_id).await?;

        let mut tx = self.pool.begin().await?;
        let result = sqlx::query("DELETE FROM messages WHERE conversation_id = ?")
            .bind(conversation_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "UPDATE conversations
             SET last_message_at = NULL, last_read_message_id = NULL, updated_at = ?
             WHERE id = ?",
        )
        .bind(now_ms())
        .bind(conversation_id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;

        self.emit(ChatEvent::MessagesCleared {
            conversation_id: conversation_id.to_owned(),
        });
        Ok(result.rows_affected())
    }

    /// 把已读水位推到最新一条 peer 消息，并返回更新后的会话摘要。
    pub async fn mark_read(&self, conversation_id: &str) -> Result<ConversationSummary> {
        self.ensure_exists(conversation_id).await?;

        let max_peer: Option<i64> = sqlx::query_scalar(
            "SELECT MAX(id) FROM messages WHERE conversation_id = ? AND sender = 'peer'",
        )
        .bind(conversation_id)
        .fetch_one(&self.pool)
        .await?;

        if let Some(max_id) = max_peer {
            sqlx::query(
                "UPDATE conversations
                 SET last_read_message_id = MAX(COALESCE(last_read_message_id, 0), ?)
                 WHERE id = ?",
            )
            .bind(max_id)
            .bind(conversation_id)
            .execute(&self.pool)
            .await?;
        }

        let last_read_message_id: Option<i64> =
            sqlx::query_scalar("SELECT last_read_message_id FROM conversations WHERE id = ?")
                .bind(conversation_id)
                .fetch_one(&self.pool)
                .await?;

        self.emit(ChatEvent::ConversationRead {
            conversation_id: conversation_id.to_owned(),
            last_read_message_id,
        });
        self.get_conversation(conversation_id).await
    }

    /// 库为空时灌入几条演示会话，方便每个 UI 框架一启动就有东西可看。
    /// 已经有数据时什么都不做，返回 0。
    pub async fn seed_demo(&self) -> Result<usize> {
        let existing: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM conversations")
            .fetch_one(&self.pool)
            .await?;
        if existing > 0 {
            return Ok(0);
        }

        let seeds: [(&str, &[(Sender, &str)]); 3] = [
            (
                "产品讨论组",
                &[
                    (Sender::Peer, "在吗？UI 框架选得怎么样了"),
                    (Sender::User, "还在对比，先把后端写好了"),
                    (Sender::Peer, "那挺快啊"),
                    (Sender::Peer, "记得测一下中文输入法"),
                ],
            ),
            (
                "妈妈",
                &[
                    (Sender::Peer, "吃饭了吗"),
                    (Sender::User, "吃了，刚吃完"),
                    (Sender::Peer, "早点睡"),
                ],
            ),
            (
                "Rust 学习小组",
                &[
                    (Sender::Peer, "今晚分享 sqlx 吗？"),
                    (Sender::Peer, "我准备了几个例子"),
                ],
            ),
        ];

        let base = now_ms() - 6 * 3_600_000;
        let mut tx = self.pool.begin().await?;

        for (index, (title, entries)) in seeds.iter().enumerate() {
            let id = uuid::Uuid::new_v4().to_string();
            // 第一个会话（带未读的那条）排在最前面，UI 一打开就能看到红点。
            let created_at = base + (seeds.len() - 1 - index) as i64 * 3_600_000;

            sqlx::query(
                "INSERT INTO conversations (id, title, created_at, updated_at) VALUES (?, ?, ?, ?)",
            )
            .bind(&id)
            .bind(*title)
            .bind(created_at)
            .bind(created_at)
            .execute(&mut *tx)
            .await?;

            let mut ids = Vec::with_capacity(entries.len());
            for (offset, (sender, content)) in entries.iter().enumerate() {
                let created = created_at + (offset as i64 + 1) * 300_000;
                let result = sqlx::query(
                    "INSERT INTO messages (conversation_id, sender, content, created_at)
                     VALUES (?, ?, ?, ?)",
                )
                .bind(&id)
                .bind(*sender)
                .bind(*content)
                .bind(created)
                .execute(&mut *tx)
                .await?;
                ids.push(result.last_insert_rowid());
            }

            let last_at = created_at + entries.len() as i64 * 300_000;
            // 第一个会话留一条未读，方便看红点效果。
            let read_marker = if index == 0 {
                ids.get(ids.len().saturating_sub(2)).copied()
            } else {
                ids.last().copied()
            };
            sqlx::query(
                "UPDATE conversations
                 SET last_message_at = ?, updated_at = ?, last_read_message_id = ?
                 WHERE id = ?",
            )
            .bind(last_at)
            .bind(last_at)
            .bind(read_marker)
            .bind(&id)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(seeds.len())
    }

    // ---------------------------------------------------------------- 内部

    async fn fetch_conversation(&self, id: &str) -> Result<Conversation> {
        sqlx::query_as::<_, Conversation>(
            "SELECT id, title, created_at, updated_at, last_message_at
             FROM conversations WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| Error::ConversationNotFound(id.to_owned()))
    }

    async fn ensure_exists(&self, id: &str) -> Result<()> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM conversations WHERE id = ?")
            .bind(id)
            .fetch_one(&self.pool)
            .await?;
        if count == 0 {
            Err(Error::ConversationNotFound(id.to_owned()))
        } else {
            Ok(())
        }
    }

    async fn latest_messages(&self) -> Result<HashMap<String, Message>> {
        let messages: Vec<Message> = sqlx::query_as(
            "SELECT id, conversation_id, sender, content, created_at FROM (
                 SELECT id, conversation_id, sender, content, created_at,
                        ROW_NUMBER() OVER (PARTITION BY conversation_id ORDER BY id DESC) AS rn
                 FROM messages
             ) WHERE rn = 1",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(messages
            .into_iter()
            .map(|message| (message.conversation_id.clone(), message))
            .collect())
    }

    /// 写一条消息并推进会话的 `last_message_at`，最后广播事件。
    async fn insert_message(
        &self,
        conversation_id: &str,
        sender: Sender,
        content: &str,
    ) -> Result<Message> {
        let now = now_ms();
        let mut tx = self.pool.begin().await?;

        let result = sqlx::query(
            "INSERT INTO messages (conversation_id, sender, content, created_at)
             VALUES (?, ?, ?, ?)",
        )
        .bind(conversation_id)
        .bind(sender)
        .bind(content)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        sqlx::query("UPDATE conversations SET last_message_at = ?, updated_at = ? WHERE id = ?")
            .bind(now)
            .bind(now)
            .bind(conversation_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        let message = Message {
            id: result.last_insert_rowid(),
            conversation_id: conversation_id.to_owned(),
            sender,
            content: content.to_owned(),
            created_at: now,
        };
        self.emit(ChatEvent::MessageCreated {
            message: message.clone(),
        });
        Ok(message)
    }

    /// 延迟一小段时间后插入一条随机回复。
    ///
    /// 语义：同一会话里只有"最后一次"发送会触发回复。用户连发几条时，
    /// 前面的回复任务会在醒来时发现代次过期而安静退出，所以机器人只会回一次。
    fn spawn_auto_reply(&self, conversation_id: String) {
        if !self.config.auto_reply.enabled {
            return;
        }

        let epoch = {
            let mut epochs = self.epochs();
            let slot = epochs.entry(conversation_id.clone()).or_insert(0);
            *slot += 1;
            *slot
        };

        let service = self.clone();
        tokio::spawn(async move {
            let delay = service.config.auto_reply.delay();
            service.emit(ChatEvent::Typing {
                conversation_id: conversation_id.clone(),
                sender: Sender::Peer,
                is_typing: true,
            });
            tokio::time::sleep(delay).await;

            if !service.is_current_reply(&conversation_id, epoch) {
                // 被更新的消息接管了：不要发 typing=false，否则会把新任务的指示器关掉。
                debug!(conversation_id = %conversation_id, "自动回复已过期，放弃发送");
                return;
            }

            let reply = service.config.auto_reply.pick_reply();
            match service
                .insert_message(&conversation_id, Sender::Peer, &reply)
                .await
            {
                Ok(message) => debug!(id = message.id, "自动回复已写入"),
                Err(error) => warn!(conversation_id = %conversation_id, %error, "自动回复写入失败"),
            }

            service.emit(ChatEvent::Typing {
                conversation_id: conversation_id.clone(),
                sender: Sender::Peer,
                is_typing: false,
            });
            service.forget_epoch(&conversation_id, epoch);
        });
    }

    fn epochs(&self) -> MutexGuard<'_, HashMap<String, u64>> {
        // 锁里只有几次 +1/查表，不可能 panic，毒化就恢复现场继续用。
        self.reply_epochs
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn is_current_reply(&self, conversation_id: &str, epoch: u64) -> bool {
        self.epochs().get(conversation_id).copied() == Some(epoch)
    }

    fn forget_epoch(&self, conversation_id: &str, epoch: u64) {
        let mut epochs = self.epochs();
        if epochs.get(conversation_id).copied() == Some(epoch) {
            epochs.remove(conversation_id);
        }
    }

    fn required_content(&self, content: &str) -> Result<String> {
        let trimmed = content.trim();
        if trimmed.is_empty() {
            return Err(Error::EmptyContent);
        }
        if trimmed.chars().count() > self.config.max_content_chars {
            return Err(Error::ContentTooLong {
                max: self.config.max_content_chars,
            });
        }
        Ok(trimmed.to_owned())
    }

    fn required_title(&self, title: &str) -> Result<String> {
        let trimmed = title.trim();
        if trimmed.is_empty() {
            return Err(Error::EmptyTitle);
        }
        if trimmed.chars().count() > self.config.max_title_chars {
            return Err(Error::TitleTooLong {
                max: self.config.max_title_chars,
            });
        }
        Ok(trimmed.to_owned())
    }

    fn default_title(&self, title: Option<&str>) -> Result<String> {
        match title.map(str::trim).filter(|title| !title.is_empty()) {
            Some(title) => self.required_title(title),
            None => Ok("新会话".to_owned()),
        }
    }
}

/// 转义 LIKE 的通配符，让 `%` / `_` 按字面量匹配。
fn escape_like(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for ch in input.chars() {
        if matches!(ch, '\\' | '%' | '_') {
            escaped.push('\\');
        }
        escaped.push(ch);
    }
    escaped
}
