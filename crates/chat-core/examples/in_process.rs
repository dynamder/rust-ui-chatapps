//! 进程内集成示例：原生 UI 框架（egui / iced / slint / gpui…）照抄这里的调用方式即可。
//!
//! ```bash
//! cargo run -p chat-core --example in_process
//! ```

use std::time::Duration;

use chat_core::{ChatService, DEFAULT_REPLIES};

#[tokio::main]
async fn main() -> chat_core::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let service = ChatService::open("data/in-process-demo.db").await?;
    println!(
        "数据库已打开，现有会话 {} 个",
        service.list_conversations().await?.len()
    );

    let conversation = service
        .create_conversation(Some("和 UI 框架的对话"))
        .await?;
    println!("新建会话 {} ({})", conversation.title, conversation.id);

    // 先订阅，再发消息，否则可能漏掉事件。
    let mut events = service.subscribe();
    let sent = service
        .send_user_message(&conversation.id, "你好，这条来自 UI 线程")
        .await?;
    println!("已发送 #{}: {}", sent.id, sent.content);

    // 事件顺序：message_created(user) -> typing(true) -> message_created(peer) -> typing(false)
    while let Ok(Ok(event)) = tokio::time::timeout(Duration::from_secs(3), events.recv()).await {
        let json = serde_json::to_string(&event).expect("事件可以序列化");
        println!("事件 {json}");
    }

    let summary = service.get_conversation(&conversation.id).await?;
    println!(
        "会话状态：{} 条消息，{} 条未读，最后一条 = {:?}",
        summary.message_count,
        summary.unread_count,
        summary
            .last_message
            .as_ref()
            .map(|message| &message.content)
    );

    let read = service.mark_read(&conversation.id).await?;
    println!("标记已读后未读数 = {}", read.unread_count);

    let page = service.messages(&conversation.id, None, Some(10)).await?;
    for message in &page.messages {
        println!("  [{}] {}", message.sender.as_str(), message.content);
    }
    println!("还有更早的消息吗？{}", page.has_more);

    let sample = service.config().auto_reply.pick_reply();
    println!("内置语料 {} 句，例如：{sample}", DEFAULT_REPLIES.len());

    Ok(())
}
