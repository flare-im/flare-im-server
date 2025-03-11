/// 消息状态枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageStatus {
    /// 初始状态，消息刚创建
    Initial = 0,

    /// 待处理状态，等待路由或重试
    Pending = 1,

    /// 已发送到路由服务
    Sent = 2,

    /// 消息已到达网关
    Received = 3,

    /// 消息已投递到接收方
    Delivered = 4,

    /// 接收方已读消息
    Read = 5,

    /// 消息发送失败
    Failed = 6,

    /// 消息已撤回
    Recalled = 7,

    /// 消息已删除
    Deleted = 8,

    /// 消息被过滤
    Filtered = 9,

    /// 消息需要立即处理
    Urgent = 10,

    /// 消息已过期
    Expired = 11,
}

impl MessageStatus {
    /// 检查消息是否处于终态
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            MessageStatus::Read
                | MessageStatus::Failed
                | MessageStatus::Recalled
                | MessageStatus::Deleted
                | MessageStatus::Filtered
                | MessageStatus::Expired
        )
    }

    /// 检查消息是否可以重试
    pub fn can_retry(&self) -> bool {
        matches!(
            self,
            MessageStatus::Initial
                | MessageStatus::Pending
                | MessageStatus::Failed
        )
    }

    /// 检查消息是否需要存储
    pub fn need_storage(&self) -> bool {
        !matches!(
            self,
            MessageStatus::Filtered
                | MessageStatus::Deleted
                | MessageStatus::Expired
        )
    }

    /// 检查消息是否需要同步
    pub fn need_sync(&self) -> bool {
        matches!(
            self,
            MessageStatus::Delivered
                | MessageStatus::Read
                | MessageStatus::Recalled
        )
    }

    /// 从i32转换为MessageStatus
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Initial),
            1 => Some(Self::Pending),
            2 => Some(Self::Sent),
            3 => Some(Self::Received),
            4 => Some(Self::Delivered),
            5 => Some(Self::Read),
            6 => Some(Self::Failed),
            7 => Some(Self::Recalled),
            8 => Some(Self::Deleted),
            9 => Some(Self::Filtered),
            10 => Some(Self::Urgent),
            11 => Some(Self::Expired),
            _ => None,
        }
    }

    /// 将MessageStatus转换为i32
    pub fn as_i32(&self) -> i32 {
        *self as i32
    }
}

impl std::fmt::Display for MessageStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageStatus::Initial => write!(f, "Initial"),
            MessageStatus::Pending => write!(f, "Pending"),
            MessageStatus::Sent => write!(f, "Sent"),
            MessageStatus::Received => write!(f, "Received"),
            MessageStatus::Delivered => write!(f, "Delivered"),
            MessageStatus::Read => write!(f, "Read"),
            MessageStatus::Failed => write!(f, "Failed"),
            MessageStatus::Recalled => write!(f, "Recalled"),
            MessageStatus::Deleted => write!(f, "Deleted"),
            MessageStatus::Filtered => write!(f, "Filtered"),
            MessageStatus::Urgent => write!(f, "Urgent"),
            MessageStatus::Expired => write!(f, "Expired"),
        }
    }
} 