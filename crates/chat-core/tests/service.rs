//! 核心库的行为测试：全部跑在内存 SQLite 上，不需要任何外部服务。

use std::time::Duration;

use chat_core::{AutoReplyConfig, ChatConfig, ChatEvent, ChatService, Error, Sender};

fn config(auto_reply: AutoReplyConfig) -> ChatConfig {
    ChatConfig {
        auto_reply,
        ..ChatConfig::default()
    }
}

async fn service(auto_reply: AutoReplyConfig) -> ChatService {
    ChatService::open_memory_with_config(config(auto_reply))
        .await
        .expect("打开内存数据库")
}

#[tokio::test]
async fn conversation_lifecycle() {
    let service = service(AutoReplyConfig::disabled()).await;

    let conversation = service.create_conversation(None).await.unwrap();
    assert_eq!(conversation.title, "新会话");
    assert!(conversation.last_message_at.is_none());

    let renamed = service
        .rename_conversation(&conversation.id, "  工作群  ")
        .await
        .unwrap();
    assert_eq!(renamed.title, "工作群");

    assert!(matches!(
        service.rename_conversation(&conversation.id, "   ").await,
        Err(Error::EmptyTitle)
    ));

    let long_title = "长".repeat(ChatConfig::default().max_title_chars + 1);
    assert!(matches!(
        service.create_conversation(Some(&long_title)).await,
        Err(Error::TitleTooLong { .. })
    ));

    let list = service.list_conversations().await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id(), conversation.id);
    assert_eq!(list[0].message_count, 0);
    assert_eq!(list[0].unread_count, 0);
    assert!(list[0].last_message.is_none());

    service.delete_conversation(&conversation.id).await.unwrap();
    assert!(matches!(
        service.get_conversation(&conversation.id).await,
        Err(Error::ConversationNotFound(_))
    ));
    assert!(matches!(
        service.delete_conversation(&conversation.id).await,
        Err(Error::ConversationNotFound(_))
    ));
}

#[tokio::test]
async fn messages_paginate_from_newest() {
    let service = service(AutoReplyConfig::disabled()).await;
    let conversation = service.create_conversation(Some("分页")).await.unwrap();

    for index in 0..25 {
        service
            .send_user_message(&conversation.id, &format!("消息 {index}"))
            .await
            .unwrap();
    }

    let first = service
        .messages(&conversation.id, None, Some(10))
        .await
        .unwrap();
    assert_eq!(first.messages.len(), 10);
    assert!(first.has_more);
    assert_eq!(first.messages.first().unwrap().content, "消息 15");
    assert_eq!(first.messages.last().unwrap().content, "消息 24");

    let second = service
        .messages(&conversation.id, first.next_before_id, Some(10))
        .await
        .unwrap();
    assert_eq!(second.messages.len(), 10);
    assert!(second.has_more);
    assert_eq!(second.messages.last().unwrap().content, "消息 14");

    let third = service
        .messages(&conversation.id, second.next_before_id, Some(10))
        .await
        .unwrap();
    assert_eq!(third.messages.len(), 5);
    assert!(!third.has_more);
    assert!(third.next_before_id.is_none());
    assert_eq!(third.messages.first().unwrap().content, "消息 0");

    // 拼起来后 id 严格递增：这就是"直接拿 id 当游标"的前提。
    let ids: Vec<i64> = third
        .messages
        .iter()
        .chain(second.messages.iter())
        .chain(first.messages.iter())
        .map(|message| message.id)
        .collect();
    assert!(ids.windows(2).all(|pair| pair[0] < pair[1]));

    // limit 超出上限时被夹住，不会一次拉爆。
    let capped = service
        .messages(&conversation.id, None, Some(10_000))
        .await
        .unwrap();
    assert_eq!(capped.messages.len(), 25);
}

#[tokio::test]
async fn search_messages_across_conversations() {
    let service = service(AutoReplyConfig::disabled()).await;
    let a = service.create_conversation(Some("A")).await.unwrap();
    let b = service.create_conversation(Some("B")).await.unwrap();

    service
        .send_user_message(&a.id, "今天加班到十点")
        .await
        .unwrap();
    service
        .send_user_message(&b.id, "今晚加班吗")
        .await
        .unwrap();
    service.send_user_message(&a.id, "进度 100%").await.unwrap();

    let hits = service.search_messages("加班", None, None).await.unwrap();
    assert_eq!(hits.len(), 2);
    assert!(hits.iter().all(|hit| hit.message.content.contains("加班")));

    let hits = service
        .search_messages("加班", Some(&a.id), None)
        .await
        .unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].conversation_title, "A");

    // LIKE 的通配符必须按字面量处理。
    let hits = service.search_messages("%", None, None).await.unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].message.content, "进度 100%");

    assert!(
        service
            .search_messages("   ", None, None)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(matches!(
        service.search_messages("x", Some("不存在"), None).await,
        Err(Error::ConversationNotFound(_))
    ));
}

#[tokio::test]
async fn auto_reply_publishes_typing_then_message() {
    let service = service(AutoReplyConfig {
        replies: vec!["收到".to_owned()],
        ..AutoReplyConfig::immediate()
    })
    .await;
    let conversation = service.create_conversation(Some("机器人")).await.unwrap();
    let mut events = service.subscribe();

    service
        .send_user_message(&conversation.id, "你好")
        .await
        .unwrap();

    let mut saw_typing = false;
    let mut reply = None;
    for _ in 0..20 {
        let event = tokio::time::timeout(Duration::from_secs(2), events.recv())
            .await
            .expect("等待事件超时")
            .expect("事件通道关闭");
        match event {
            ChatEvent::Typing {
                is_typing: true,
                sender: Sender::Peer,
                ..
            } => saw_typing = true,
            ChatEvent::MessageCreated { message } if message.sender == Sender::Peer => {
                reply = Some(message);
                break;
            }
            _ => {}
        }
    }

    assert!(saw_typing, "应当先收到 typing(true)");
    assert_eq!(reply.expect("应当收到对方消息").content, "收到");

    let summary = service.get_conversation(&conversation.id).await.unwrap();
    assert_eq!(summary.message_count, 2);
    assert_eq!(summary.unread_count, 1);
    assert_eq!(summary.last_message.as_ref().unwrap().sender, Sender::Peer);

    let summary = service.mark_read(&conversation.id).await.unwrap();
    assert_eq!(summary.unread_count, 0);
}

#[tokio::test]
async fn newer_message_supersedes_pending_reply() {
    let mut reply_config = AutoReplyConfig::immediate();
    reply_config.min_delay = Duration::from_millis(120);
    reply_config.max_delay = Duration::from_millis(120);

    let service = service(reply_config).await;
    let conversation = service.create_conversation(Some("连发")).await.unwrap();

    service
        .send_user_message(&conversation.id, "第一条")
        .await
        .unwrap();
    service
        .send_user_message(&conversation.id, "第二条")
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(600)).await;

    let page = service
        .messages(&conversation.id, None, None)
        .await
        .unwrap();
    let replies = page
        .messages
        .iter()
        .filter(|message| message.sender == Sender::Peer)
        .count();
    assert_eq!(replies, 1, "连发两条也只会回复一次");
    assert_eq!(page.messages.len(), 3);
}

#[tokio::test]
async fn clear_history_and_cascade_delete() {
    let service = service(AutoReplyConfig::disabled()).await;
    let conversation = service.create_conversation(Some("清理")).await.unwrap();

    for index in 0..3 {
        service
            .send_user_message(&conversation.id, &format!("第 {index} 条"))
            .await
            .unwrap();
    }

    assert_eq!(service.clear_messages(&conversation.id).await.unwrap(), 3);
    let page = service
        .messages(&conversation.id, None, None)
        .await
        .unwrap();
    assert!(page.messages.is_empty());

    let summary = service.get_conversation(&conversation.id).await.unwrap();
    assert_eq!(summary.message_count, 0);
    assert!(summary.last_message.is_none());

    service
        .send_user_message(&conversation.id, "清空后还能继续聊")
        .await
        .unwrap();
    service.delete_conversation(&conversation.id).await.unwrap();

    let remaining: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM messages")
        .fetch_one(service.pool())
        .await
        .unwrap();
    assert_eq!(remaining, 0, "外键级联应当删掉消息");
}

#[tokio::test]
async fn rejects_empty_and_oversized_messages() {
    let service = service(AutoReplyConfig::disabled()).await;
    let conversation = service.create_conversation(Some("校验")).await.unwrap();

    assert!(matches!(
        service.send_user_message(&conversation.id, "   ").await,
        Err(Error::EmptyContent)
    ));

    let long = "字".repeat(ChatConfig::default().max_content_chars + 1);
    assert!(matches!(
        service.send_user_message(&conversation.id, &long).await,
        Err(Error::ContentTooLong { .. })
    ));

    assert!(matches!(
        service.messages("missing", None, None).await,
        Err(Error::ConversationNotFound(_))
    ));
}

#[tokio::test]
async fn data_survives_reopen() {
    let dir = tempfile::tempdir().expect("创建临时目录");
    let path = dir.path().join("chat.db");

    let service = ChatService::open_with_config(&path, config(AutoReplyConfig::disabled()))
        .await
        .unwrap();
    let conversation = service.create_conversation(Some("持久化")).await.unwrap();
    service
        .send_user_message(&conversation.id, "写在磁盘上")
        .await
        .unwrap();
    drop(service);

    let reopened = ChatService::open_with_config(&path, config(AutoReplyConfig::disabled()))
        .await
        .unwrap();
    let page = reopened
        .messages(&conversation.id, None, None)
        .await
        .unwrap();
    assert_eq!(page.messages.len(), 1);
    assert_eq!(page.messages[0].content, "写在磁盘上");
}

#[tokio::test]
async fn seed_is_idempotent() {
    let service = service(AutoReplyConfig::disabled()).await;

    assert_eq!(service.seed_demo().await.unwrap(), 3);
    assert_eq!(service.seed_demo().await.unwrap(), 0);

    let list = service.list_conversations().await.unwrap();
    assert_eq!(list.len(), 3);
    assert!(list.iter().all(|summary| summary.last_message.is_some()));
    assert_eq!(
        list.iter()
            .filter(|summary| summary.unread_count > 0)
            .count(),
        1,
        "演示数据里应当正好有一个带未读的会话"
    );
}
