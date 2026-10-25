//! 聊天后端核心：用 sqlx + SQLite 存会话与消息，并带一个"随机回复"机器人。
//!
//! 设计目标是被任意 UI 框架直接复用：
//!
//! - 所有 API 都是 `async` 的，需要调用方自己提供 tokio 运行时；
//! - 没有全局状态，[`ChatService`] 实现了 `Clone`，可以直接塞进框架的状态容器；
//! - 数据变更通过 [`ChatService::subscribe`] 的广播通道推给 UI，
//!   同时也保留了同步查询接口，方便只想轮询的框架。
//!
//! ```no_run
//! # async fn demo() -> chat_core::Result<()> {
//! use chat_core::ChatService;
//!
//! let service = ChatService::open("data/chat.db").await?;
//! let conversation = service.create_conversation(Some("和 UI 框架的对话")).await?;
//! let mut events = service.subscribe();
//!
//! service.send_user_message(&conversation.id, "你好").await?;
//!
//! while let Ok(event) = events.recv().await {
//!     println!("{event:?}");
//! }
//! # Ok(()) }
//! ```

mod config;
mod db;
mod error;
mod event;
mod model;
mod service;

pub use config::{AutoReplyConfig, ChatConfig, DEFAULT_REPLIES};
pub use db::MIGRATOR;
pub use error::{Error, Result};
pub use event::ChatEvent;
pub use model::{Conversation, ConversationSummary, Message, MessageHit, MessagePage, Sender};
pub use service::ChatService;
