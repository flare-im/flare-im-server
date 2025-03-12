/// Kafka 主题常量定义
pub struct KafkaTopics;

impl KafkaTopics {
    /// 消息存储主题
    pub const MESSAGE_STORE: &'static str = "message_store";
    
    /// 消息分发主题
    pub const MESSAGE_DISTRIBUTION: &'static str = "message_distribution";
    
    /// 离线通知主题
    pub const OFFLINE_NOTIFICATIONS: &'static str = "offline_notifications";
    
    /// 消息状态主题
    pub const MESSAGE_STATUS: &'static str = "message_status";
    
    /// 死信队列主题
    pub const DEAD_LETTER: &'static str = "dead_letter";
}
