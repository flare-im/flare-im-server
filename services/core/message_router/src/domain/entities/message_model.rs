
/// 用户状态
#[derive(Debug, Clone)]
pub struct UserStatus {
    /// 用户ID
    pub user_id: String,
    /// 是否被封禁
    pub is_banned: bool,
    /// 封禁原因
    pub ban_reason: Option<String>,
    /// 封禁时间
    pub ban_time: Option<i64>,
    /// 封禁截止时间
    pub ban_expire_time: Option<i64>,
    /// 用户状态（在线/离线/隐身等）
    pub status: i32,
    /// 最后活跃时间
    pub last_active_time: i64,
}

/// 设备状态
#[derive(Debug, Clone)]
pub struct DeviceStatus {
    /// 设备ID
    pub device_id: String,
    /// 是否被封禁
    pub is_banned: bool,
    /// 封禁原因
    pub ban_reason: Option<String>,
    /// 封禁时间
    pub ban_time: Option<i64>,
    /// 封禁截止时间
    pub ban_expire_time: Option<i64>,
    /// 设备类型
    pub device_type: i32,
    /// 设备名称
    pub device_name: String,
    /// 最后活跃时间
    pub last_active_time: i64,
}



/// 群成员状态
#[derive(Debug, Clone)]
pub struct MemberStatus {
    /// 是否为群成员
    pub is_member: bool,
    /// 是否被禁言
    pub is_muted: bool,
    /// 禁言截止时间
    pub mute_expire_time: Option<i64>,
    /// 成员角色（0:普通成员 1:管理员 2:群主）
    pub role: i32,
    /// 加入时间
    pub join_time: Option<i64>,
    /// 最后发言时间
    pub last_speak_time: Option<i64>,
    /// 群昵称
    pub nickname: Option<String>,
}

/// 群状态
#[derive(Debug, Clone)]
pub struct GroupStatus {
    /// 群ID
    pub group_id: String,
    /// 群名称
    pub name: String,
    /// 群头像
    pub avatar: Option<String>,
    /// 群简介
    pub description: Option<String>,
    /// 群主ID
    pub owner_id: String,
    /// 是否活跃
    pub is_active: bool,
    /// 是否被禁言
    pub is_muted: bool,
    /// 禁言截止时间
    pub mute_expire_time: Option<i64>,
    /// 创建时间
    pub create_time: i64,
    /// 更新时间
    pub update_time: i64,
}


/// 好友关系状态
#[derive(Debug, Clone)]
pub struct Friendship {
    /// 是否互为好友
    pub is_friend: bool,
    /// 是否在黑名单中
    pub in_blacklist: bool,
    /// 好友关系状态（0:无关系 1:待确认 2:已确认 3:已拒绝）
    pub status: i32,
    /// 好友备注
    pub remark: Option<String>,
    /// 好友分组
    pub group: Option<String>,
    /// 好友来源
    pub source: Option<String>,
    /// 添加时间
    pub add_time: Option<i64>,
    /// 更新时间
    pub update_time: Option<i64>,
}
