# chatapp 后端 · Rust + sqlx + SQLite

给 4 个候选 Rust UI 框架（dioxus / iced / freya / gpui-kit）共用的聊天后端。
UI 侧只负责画界面，会话管理、消息落库、历史分页、搜索、自动回复都在这里。

```
chatapps/
├── Cargo.toml                  # workspace
├── crates/
│   ├── chat-core/              # 核心库：sqlx + SQLite
│   │   ├── migrations/         # schema，启动时自动执行
│   │   ├── examples/in_process.rs
│   │   └── tests/service.rs
│   └── chat-server/            # HTTP 层：REST + SSE，既是 bin 也是 lib
│       ├── src/lib.rs          # 可嵌入的 router()
│       ├── src/main.rs         # 独立进程
│       └── examples/embed.rs   # 嵌进 webview 应用
└── README.md
```

将来每个 UI 试验品作为新的 workspace 成员加进来，例如 `crates/ui-iced`，
`members = [..., "crates/ui-iced"]`，然后依赖 `chat-core`（进程内）或请求 `chat-server`。

---

## 快速开始

```bash
# 起服务：--seed 会在空库里写 3 个演示会话
cargo run -p chat-server -- --seed
# chat-server 已启动
#   地址    : http://127.0.0.1:8787
#   数据库  : data/chat.db
#   事件流  : http://127.0.0.1:8787/api/events
```

```bash
# 会话列表（带最后一条消息、未读数）
curl -s localhost:8787/api/conversations | jq

# 发一条消息：立刻返回用户消息，0.6~1.4s 后机器人随机回一句
CONV=$(curl -s localhost:8787/api/conversations | jq -r '.[0].id')
curl -s -X POST "localhost:8787/api/conversations/$CONV/messages" \
  -H 'content-type: application/json' \
  -d '{"content":"你好"}' | jq

# 监听事件（另开一个终端）：能看到 typing 和新消息
curl -N localhost:8787/api/events
```

进程内跑一遍完整流程：

```bash
cargo run -p chat-core --example in_process
```

服务参数：

| 参数 | 默认值 | 说明 |
| --- | --- | --- |
| `--db` | `data/chat.db` | SQLite 文件，目录不存在会自动创建 |
| `--addr` | `127.0.0.1:8787` | 监听地址 |
| `--seed` | 关 | 空库时写入演示会话（已有数据则跳过） |
| `--no-auto-reply` | 关 | 关闭机器人，方便只测 UI 渲染 |
| `--reply-min-ms` / `--reply-max-ms` | 600 / 1400 | 自动回复延迟区间 |

---

## 四个候选框架怎么接

四个框架分两派：dioxus 的 UI 跑在 webview / wasm 里，够不到 Rust 侧的 sqlx，只能走 HTTP；
另外三个是原生渲染，可以把 `chat-core` 直接链进去。

| 框架 | UI 跑在哪 | 后端形态 | 必须满足的前提 |
| --- | --- | --- | --- |
| dioxus | web(wasm) 或 desktop(webview) | HTTP + SSE | 起 `chat-server`，或把 `chat_server::router` 嵌进 desktop 进程 |
| iced | 原生 | 进程内 `chat-core` | 开 `iced` 的 `tokio` feature，`Task` 才跑在 tokio 上 |
| freya | 原生（0.4 起自有响应式核心） | 进程内 `chat-core` | `main()` 里建 tokio runtime 并 `rt.enter()`，guard 活到程序结束 |
| gpui-kit | 原生 | 进程内 `chat-core` | gpui executor 没有 tokio 上下文，DB 调用放独立 runtime |

### dioxus：HTTP + SSE

web 形态起独立服务：

```bash
cargo run -p chat-server -- --seed
```

desktop 想做成**单二进制、单进程**（不额外跑 chat-server），就把 router 嵌进去：

```rust
let service = ChatService::open("data/chat.db").await?;
let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
let port = listener.local_addr()?.port();          // 让系统分配端口
tokio::spawn(axum::serve(listener, chat_server::router(service)));
// 前端统一用 http://127.0.0.1:{port}/api/...
```

前端侧（web 和 desktop 共用同一套代码）：

```rust
// 列表：use_resource + reqwest（wasm 下走 fetch）
let conversations = use_resource(move || async move {
    reqwest::get(format!("{base}/api/conversations"))
        .await?
        .json::<Vec<ConversationSummary>>()
        .await
});

// 事件：SSE，注意我们的事件名是 chat
use_hook(|| {
    let source = web_sys::EventSource::new(&format!("{base}/api/events")).unwrap();
    let callback = Closure::<dyn FnMut(web_sys::MessageEvent)>::new(move |event: web_sys::MessageEvent| {
        if let Some(text) = event.data().as_string() {
            // serde_json::from_str::<ChatEvent>(&text) 之后塞进 signal
        }
    });
    source
        .add_event_listener_with_callback("chat", callback.as_ref().unchecked_ref())
        .unwrap();
    callback.forget(); // 生产代码请配合 use_drop 取消订阅，别泄漏
});
```

两个坑：

- `EventSource` 不能加自定义 header —— 我们的接口也不需要认证，没问题；
- webview 里 `127.0.0.1` 和 `localhost` 是不同 origin，`base` 统一一个即可。
  CORS 已 `permissive`，`http://localhost:8080` 这类开发服务器能直接调。

### iced：进程内

```toml
iced = { version = "0.14", features = ["tokio"] }
```

`tokio` feature 让 iced 用 tokio 当 `Task` 的执行器，于是可以直接 await `chat-core`：

```rust
fn update(&mut self, message: Message) -> Task<Message> {
    match message {
        Message::Send => {
            let service = self.service.clone();
            let conversation = self.active.clone();
            let content = std::mem::take(&mut self.draft);
            Task::perform(
                async move { service.send_user_message(&conversation, &content).await },
                Message::Sent,
            )
        }
        Message::Sent(Ok(message)) => {
            self.messages.push(message);
            Task::none()
        }
        Message::Sent(Err(error)) => {
            self.error = Some(error.to_string());
            Task::none()
        }
        _ => Task::none(),
    }
}
```

事件流用 `Subscription` 包 `BroadcastStream`（广播通道本身不依赖 runtime，被 iced 的 executor 轮询即可）：

```rust
fn subscription(&self) -> Subscription<Message> {
    let service = self.service.clone();
    Subscription::run_with_id("chat-events", move || {
        BroadcastStream::new(service.subscribe()).filter_map(|event| async move {
            match event {
                Ok(ChatEvent::MessageCreated { message }) => Some(Message::Incoming(message)),
                Ok(ChatEvent::Typing { conversation_id, is_typing: true, .. }) => {
                    Some(Message::Typing(conversation_id))
                }
                _ => None,
            }
        })
    })
}
```

`run_with_id` 的 id 需要 `Hash`，闭包签名以你用的 iced 版本为准。

### freya：进程内 + `rt.enter()`

按官方 tokio 集成文档的写法，`main()` 里建 runtime、`enter()` 并保活，之后 freya
自己的 `spawn` / `use_future` 里就能直接 await sqlx：

```rust
fn main() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    // launch 之前先把库打开，block_on 在这里是安全的
    let service = runtime.block_on(ChatService::open("data/chat.db")).unwrap();
    let _guard = runtime.enter(); // 不能 drop，必须活到程序结束

    launch(LaunchConfig::new().with_window(WindowConfig::new_app(App { service })));
}
```

组件里：

```rust
let service = app.service.clone();
use_future(move || {
    let service = service.clone();
    async move {
        let list = service.list_conversations().await.unwrap();
        // 写回 use_state
    }
});
```

freya 的响应式核心是单线程的：不要从别的线程改 state。确实要跨线程时，
用 `watch` channel + `use_track_watcher`（需要 `sdk-tokio` feature）。

### gpui-kit：进程内 + 后台 tokio runtime

gpui 有自己的 foreground / background executor，但**不提供 tokio 上下文**，
而 sqlx 的 future 必须跑在 tokio runtime 上。做法是独立 runtime + channel 回前台：

```rust
struct ChatApp {
    service: ChatService,
    runtime: tokio::runtime::Handle,
    messages: Vec<Message>,
}

impl ChatApp {
    fn send(&mut self, cx: &mut Context<Self>) {
        let (service, runtime) = (self.service.clone(), self.runtime.clone());
        let (conversation, content) = (self.active.clone(), std::mem::take(&mut self.draft));
        let (tx, rx) = futures::channel::oneshot::channel();

        // DB 操作丢到 tokio runtime
        runtime.spawn(async move {
            let _ = tx.send(service.send_user_message(&conversation, &content).await);
        });

        // 前台等结果，再 notify
        cx.spawn(async move |this, cx| {
            if let Ok(result) = rx.await {
                this.update(cx, |this, cx| {
                    if let Ok(message) = result {
                        this.messages.push(message);
                    }
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
    }
}
```

- `runtime` 在 `main()` 里建一次，只存 `Handle`，别在 UI 线程 `block_on`；
- 别在 gpui 的 async 上下文里用 `tokio::time::sleep`（用 `cx.background_executor().timer()`）。
  我们库内部的 sleep 跑在自己的 runtime 上，不受影响；
- 事件流同理：`service.subscribe()` 收到 `ChatEvent` 后用 mpsc 转给前台，
  `cx.spawn` 里 `await` 之后 `cx.notify()`。

### 四个框架通用的注意点

- `ChatService` 是 `Clone` 的（内部只有连接池和 `Arc`），放进哪个框架的状态容器都行；
- 自动回复在后台 `tokio::spawn`，所以**调用线程必须有 tokio 上下文**：
  iced 开 `tokio` feature、freya `rt.enter()`、gpui 用独立 runtime；
- 事件通道 1024 条，UI 跟不上会收到 `Lagged`。事件只是通知，重新
  `list_conversations()` / `messages()` 即可；
- 中文字形依赖系统字体：iced / freya / gpui 都走系统字体回退，精简镜像里
  缺 CJK 字体时会显示方块，装一个 Noto Sans CJK 就好；
- 时间戳是 Unix 毫秒，格式化交给 UI。

---

## 数据模型

```sql
conversations (
  id                   TEXT PK,   -- UUID v4
  title                TEXT,
  created_at           INTEGER,   -- Unix 毫秒
  updated_at           INTEGER,   -- 重命名 / 新消息时更新
  last_message_at      INTEGER,   -- 列表排序用，空会话为 NULL
  last_read_message_id INTEGER    -- 已读水位
)

messages (
  id              INTEGER PK AUTOINCREMENT,  -- 单调递增
  conversation_id TEXT FK -> conversations(id) ON DELETE CASCADE,
  sender          TEXT,  -- 'user' | 'peer'
  content         TEXT,
  created_at      INTEGER
)
```

三条约定，UI 侧最好照着来：

1. **时间是 Unix 毫秒（`i64`）**，不是字符串，时区/格式由 UI 自己决定；
2. **`messages.id` 单调递增**，既可以当排序依据，也可以直接当分页游标（不需要 offset）；
3. **未读数 = `id > last_read_message_id` 的 peer 消息条数**，用户自己发的消息不算未读。

会话 id 用 UUID、消息 id 用自增整数是有意为之：会话数量少且由客户端创建/引用，
UUID 不会冲突；消息量大且要"向上翻页"，自增 id 是最稳的游标。

---

## 事件

所有写操作都会广播事件，序列化后带 `type` 字段（`snake_case`）：

| `type` | 附带字段 | 什么时候发 |
| --- | --- | --- |
| `conversation_created` | `conversation` | 新建会话 |
| `conversation_updated` | `conversation` | 重命名 |
| `conversation_deleted` | `conversation_id` | 删除会话 |
| `messages_cleared` | `conversation_id` | 清空历史 |
| `conversation_read` | `conversation_id`, `last_read_message_id` | 标记已读 |
| `message_created` | `message` | 任一方发消息 |
| `typing` | `conversation_id`, `sender`, `is_typing` | 机器人开始/结束"输入" |

```json
{"type":"message_created","message":{"id":42,"conversation_id":"…","sender":"peer","content":"收到收到","created_at":1760000000000}}
{"type":"typing","conversation_id":"…","sender":"peer","is_typing":true}
```

一次"用户发消息 → 机器人回复"的完整事件顺序：

```
message_created(user) → typing(true) → message_created(peer) → typing(false)
```

---

## REST API

| 方法 | 路径 | 说明 | 成功 |
| --- | --- | --- | --- |
| GET | `/api/health` | 探活，返回 `{ok, version}` | 200 |
| GET | `/api/conversations` | 会话列表，按最后活跃倒序 | 200 |
| POST | `/api/conversations` | 新建会话，body 可省略：`{"title":"..."}` | 201 |
| GET | `/api/conversations/{id}` | 单个会话摘要 | 200 |
| PATCH | `/api/conversations/{id}` | 重命名：`{"title":"..."}` | 200 |
| DELETE | `/api/conversations/{id}` | 删除会话（消息级联删除） | 204 |
| GET | `/api/conversations/{id}/messages?before_id=&limit=` | 一页历史消息 | 200 |
| POST | `/api/conversations/{id}/messages` | 发消息：`{"content":"..."}`，触发自动回复 | 201 |
| DELETE | `/api/conversations/{id}/messages` | 清空历史，返回 `{deleted:n}` | 200 |
| POST | `/api/conversations/{id}/read` | 标记已读，返回更新后的会话摘要 | 200 |
| GET | `/api/search?q=&conversation_id=&limit=` | 消息搜索，`conversation_id` 可选 | 200 |
| GET | `/api/events?conversation_id=` | SSE 事件流，`conversation_id` 可选 | 200 |

会话摘要长这样（`last_message` 直接嵌在列表项里，不用二次请求）：

```json
{
  "id": "…", "title": "产品讨论组",
  "created_at": 1760000000000, "updated_at": 1760000180000, "last_message_at": 1760000180000,
  "last_message": {"id": 4, "conversation_id": "…", "sender": "peer", "content": "记得测一下中文输入法", "created_at": 1760000180000},
  "message_count": 4, "unread_count": 1
}
```

消息分页（`messages` 内部按时间升序，UI 直接从上往下渲染；往上翻页时把 `next_before_id`
作为下一次的 `before_id`）：

```json
{"messages": [ … ], "has_more": true, "next_before_id": 15}
```

错误统一为：

```json
{"error": {"code": "not_found", "message": "会话不存在: 3f2a…"}}
```

`code` 取值：`not_found` / `invalid_input` / `invalid_query` / `internal`。
`invalid_input` 对应空内容、超长内容、空标题这些可以直接显示给用户的校验错误。

---

## 设计取舍

- **搜索用 `LIKE` 而不是 FTS5。** SQLite 默认的 `unicode61` 分词器不切中文，
  “加班”这类查询会整句当一个 token；`trigram` 分词又要求至少 3 个字符。
  本地几百到几万条消息，`LIKE '%…%'` 配合会话索引完全够快，行为也和用户直觉一致。
  真要做大再换 FTS5 + jieba 分词的影子表。
- **自动回复会合并连发。** 每个会话有一个"回复代次"，新消息会把代次 +1；
  先醒来的旧任务发现代次过期就安静退出。所以连发 3 条只会得到 1 条回复，
  也更像真人（不会一句一句复读）。
- **先提交事务再广播事件。** 事件里携带的是已落库的数据，UI 拿到就能直接渲染，
  不会出现"事件到了但查库查不到"。代价是极端情况下事件可能丢（进程崩溃），
  所以事件只当通知、查询接口才是事实来源。
- **HTTP 层同时是 lib。** dioxus web 用独立进程，dioxus desktop 把 `router()` 挂到
  `127.0.0.1:0` 上，发布时单二进制、单进程；两种形态共用同一套前端代码。
- **数据库层不用编译期校验的 `query!` 宏。** 那种写法要求构建时有 `DATABASE_URL` 或
  `.sqlx` 离线缓存，会让 4 个 UI crate 的构建都依赖环境。这里全部用运行时查询，
  `cargo build` 只需要网络拉依赖。
- **时间戳存 `i64` 毫秒。** 不引入 chrono，也不把格式化责任推给后端。
- **没有 users 表。** 单机单用户 + 一个机器人，`sender` 两态足够测 UI；
  要多人群聊再加 `users` / `conversation_members`，`sender` 换成 user_id。
- **删除是硬删除。** 演示项目不需要回收站；要软删除加 `deleted_at` 再过滤即可。

## 测试

```bash
cargo test -p chat-core
```

全部跑在内存 SQLite 上，不依赖任何外部服务，覆盖：

- 会话创建 / 重命名 / 删除 / 标题与内容校验
- 消息分页（游标、`has_more`、顺序、limit 上限）
- 中文子串搜索、按会话过滤、LIKE 通配符转义
- 自动回复的事件顺序、未读数、`mark_read`
- 连发消息时旧回复被取消
- 清空历史、删除会话的级联删除
- 关闭再打开后数据仍在
- `seed_demo` 幂等

## 还没做的

附件/图片、多用户与群聊、已读回执（逐条）、消息编辑与撤回、FTS5 全文索引、
会话置顶/归档、软删除、离线队列与重试。需要哪个说一声，加在 `chat-core` 里即可。
