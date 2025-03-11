/// 预处理状态码
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreProcessCode {
    /// 校验通过
    Ok = 0,
    /// 消息格式错误
    InvalidFormat = 1,
    /// 内容违规
    InvalidContent = 2,

    // 权限相关错误 (10-29)
    /// 非好友关系
    NotFriend = 10,
    /// 在对方黑名单中
    InBlacklist = 11,
    /// 不是群成员
    NotGroupMember = 12,
    /// 已被禁言
    Muted = 13,
    /// 群已被禁言
    GroupMuted = 14,
    /// 已被踢出群
    Kicked = 15,
    /// 群已解散
    GroupDissolved = 16,
    /// 用户已被封禁
    UserBanned = 17,
    /// 设备已被封禁
    DeviceBanned = 18,

    // 业务规则限制 (30-49)
    /// 发送频率超限
    FrequencyLimit = 30,
    /// 单聊消息数量超限
    PrivateMessageLimit = 31,
    /// 群消息数量超限
    GroupMessageLimit = 32,
    /// 消息长度超限
    ContentLengthLimit = 33,
    /// 附件大小超限
    AttachmentSizeLimit = 34,

    // 系统错误 (50+)
    /// 系统内部错误
    SystemError = 50,
    /// 服务暂时不可用
    ServiceUnavailable = 51,
    /// 数据库错误
    DatabaseError = 52,
    /// 缓存错误
    CacheError = 53,
}

impl PreProcessCode {
    pub fn as_i32(&self) -> i32 {
        *self as i32
    }

    /// 获取错误描述
    pub fn description(&self) -> &'static str {
        match self {
            Self::Ok => "成功",
            Self::InvalidFormat => "消息格式错误",
            Self::InvalidContent => "内容违规",
            Self::NotFriend => "不是好友关系",
            Self::InBlacklist => "在对方黑名单中",
            Self::NotGroupMember => "不是群成员",
            Self::Muted => "已被禁言",
            Self::GroupMuted => "群已被禁言",
            Self::Kicked => "已被踢出群",
            Self::GroupDissolved => "群已解散",
            Self::UserBanned => "用户已被封禁",
            Self::DeviceBanned => "设备已被封禁",
            Self::FrequencyLimit => "发送频率超限",
            Self::PrivateMessageLimit => "单聊消息数量超限",
            Self::GroupMessageLimit => "群消息数量超限",
            Self::ContentLengthLimit => "消息长度超限",
            Self::AttachmentSizeLimit => "附件大小超限",
            Self::SystemError => "系统内部错误",
            Self::ServiceUnavailable => "服务暂时不可用",
            Self::DatabaseError => "数据库错误",
            Self::CacheError => "缓存错误",
        }
    }

    /// 是否是权限相关错误
    pub fn is_permission_error(&self) -> bool {
        matches!(
            self,
            Self::NotFriend
                | Self::InBlacklist
                | Self::NotGroupMember
                | Self::Muted
                | Self::GroupMuted
                | Self::Kicked
                | Self::GroupDissolved
                | Self::UserBanned
                | Self::DeviceBanned
        )
    }

    /// 是否是业务规则限制
    pub fn is_business_limit(&self) -> bool {
        matches!(
            self,
            Self::FrequencyLimit
                | Self::PrivateMessageLimit
                | Self::GroupMessageLimit
                | Self::ContentLengthLimit
                | Self::AttachmentSizeLimit
        )
    }

    /// 是否是系统错误
    pub fn is_system_error(&self) -> bool {
        matches!(
            self,
            Self::SystemError
                | Self::ServiceUnavailable
                | Self::DatabaseError
                | Self::CacheError
        )
    }
}

/// 消息处理结果
#[derive(Debug)]
pub struct MessageProcessResult {
    /// 消息ID
    pub message_id: String,
    /// 是否处理成功
    pub success: bool,
    /// 错误信息
    pub error: Option<String>,
    /// 路由信息
    pub routes: Vec<String>,
}
