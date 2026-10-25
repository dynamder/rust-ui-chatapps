//! 把 HTTP 层嵌进自己的进程，而不是另起 `chat-server`。
//!
//! 这是 dioxus desktop（webview）想要的形态：发布时只有一个二进制，
//! 端口用 0 让系统分配，再把地址交给前端。
//!
//! ```bash
//! cargo run -p chat-server --example embed
//! ```

use chat_core::ChatService;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let service = ChatService::open("data/embedded-demo.db").await?;
    if service.list_conversations().await?.is_empty() {
        service.seed_demo().await?;
    }

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    println!("内嵌 server 已启动: http://{address}");
    println!("把这句话喂给 webview 里的前端代码即可。");

    axum::serve(listener, chat_server::router(service))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;

    Ok(())
}
