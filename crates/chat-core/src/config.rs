use std::time::Duration;

use rand::Rng;

/// 默认中文语料：短句、口语、长度参差，方便观察气泡宽度与换行。
pub const DEFAULT_REPLIES: &[&str] = &[
    "在的，怎么了？",
    "哈哈哈哈真的假的",
    "我也这么觉得",
    "稍等，我看一下",
    "有点道理",
    "今天好累啊",
    "嗯嗯，然后呢？",
    "这个我熟",
    "晚点聊，我先忙一下",
    "你说得对",
    "收到收到",
    "别急，慢慢说",
    "我马上回你",
    "刚才没看到消息",
    "哈哈哈笑死",
    "好呀，听你的",
    "真的吗？细说",
    "行，就这么定了",
    "？？？",
    "有点意思",
    "那必须的",
    "明天再说吧",
    "我刚到家",
    "吃了吗？",
];

/// 自动回复机器人的行为。
#[derive(Debug, Clone)]
pub struct AutoReplyConfig {
    pub enabled: bool,
    /// 延迟区间：模拟对方打字，给 UI 展示“正在输入”的机会。
    pub min_delay: Duration,
    pub max_delay: Duration,
    /// 随机抽取的语料。
    pub replies: Vec<String>,
}

impl Default for AutoReplyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_delay: Duration::from_millis(600),
            max_delay: Duration::from_millis(1400),
            replies: DEFAULT_REPLIES
                .iter()
                .map(|line| (*line).to_owned())
                .collect(),
        }
    }
}

impl AutoReplyConfig {
    /// 不做任何延迟，测试里用。
    pub fn immediate() -> Self {
        Self {
            min_delay: Duration::ZERO,
            max_delay: Duration::ZERO,
            ..Self::default()
        }
    }

    /// 关掉机器人。
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Self::default()
        }
    }

    /// 在 `[min_delay, max_delay]` 里随机取一个延迟。
    pub fn delay(&self) -> Duration {
        let min = self.min_delay.as_millis() as u64;
        let max = (self.max_delay.as_millis() as u64).max(min);
        if min == max {
            return Duration::from_millis(min);
        }
        Duration::from_millis(rand::rng().random_range(min..=max))
    }

    /// 随机挑一句话。语料为空时退回内置的第一句，保证永远有回复。
    pub fn pick_reply(&self) -> String {
        if self.replies.is_empty() {
            return DEFAULT_REPLIES[0].to_owned();
        }
        let index = rand::rng().random_range(0..self.replies.len());
        self.replies[index].clone()
    }
}

/// 核心库的全部可调参数。
#[derive(Debug, Clone)]
pub struct ChatConfig {
    pub auto_reply: AutoReplyConfig,
    /// 单条消息最大字符数（按 Unicode 字符算，不是字节）。
    pub max_content_chars: usize,
    pub max_title_chars: usize,
    pub default_page_size: i64,
    pub max_page_size: i64,
    pub max_search_results: i64,
}

impl Default for ChatConfig {
    fn default() -> Self {
        Self {
            auto_reply: AutoReplyConfig::default(),
            max_content_chars: 4000,
            max_title_chars: 64,
            default_page_size: 30,
            max_page_size: 200,
            max_search_results: 100,
        }
    }
}
