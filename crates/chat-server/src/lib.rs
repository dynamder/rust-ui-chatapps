//! chat-server 有两副面孔：
//!
//! 1. 独立进程：`cargo run -p chat-server -- --seed`，给浏览器里的 dioxus web 用；
//! 2. 可嵌入的库：dioxus desktop 这类 webview 应用可以把 [`router`] 挂到
//!    `127.0.0.1:0` 上再把端口告诉前端，这样发布时只有一个二进制、不用另起进程。
//!
//! ```no_run
//! # async fn demo() -> anyhow::Result<()> {
//! use chat_core::{ChatConfig, ChatService};
//!
//! let service = ChatService::open_with_config("data/chat.db", ChatConfig::default()).await?;
//! let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
//! let address = listener.local_addr()?;
//! tokio::spawn(async move {
//!     axum::serve(listener, chat_server::router(service)).await
//! });
//! // 之后 webview 里访问 http://{address}
//! # Ok(()) }
//! ```

pub mod api;

pub use api::{AppState, router};
