use async_trait::async_trait;
use crate::domain::entities::Message;
use anyhow::Result;
use crate::entities::{GroupStatus, MemberStatus};

/// 群成员分页结果
pub struct GroupMemberPage {
    pub members: Vec<String>,     // 成员ID列表
    pub total: i64,              // 总成员数
    pub cursor: Option<String>,  // 下一页游标
    pub has_more: bool,         // 是否还有更多
}

/// 群成员查询参数
pub struct GroupMemberQuery {
    pub page_size: i32,         // 每页大小
    pub cursor: Option<String>, // 分页游标
    pub role_filter: Option<i32>, // 角色过滤
    pub active_only: bool,      // 是否只返回活跃成员
}

/// 群仓储接口
#[async_trait]
pub trait GroupRepository: Send + Sync {
    /// 检查群成员状态
    /// 
    /// # 参数
    /// * `group_id` - 群ID
    /// * `user_id` - 用户ID
    /// 
    /// # 返回
    /// * `Result<MemberStatus, Error>` - 群成员状态
    async fn check_member_status(&self, group_id: &str, user_id: &str) -> Result<MemberStatus>;

    /// 分页获取群成员列表
    /// 
    /// # 参数
    /// * `group_id` - 群ID
    /// * `query` - 查询参数
    /// 
    /// # 返回
    /// * `Result<GroupMemberPage, Error>` - 群成员分页结果
    async fn get_group_members_paged(&self, group_id: &str, query: GroupMemberQuery) -> Result<GroupMemberPage>;

    /// 获取群在线成员列表（用于消息推送优化）
    /// 
    /// # 参数
    /// * `group_id` - 群ID
    /// * `page_size` - 每页大小
    /// * `cursor` - 分页游标
    /// 
    /// # 返回
    /// * `Result<GroupMemberPage, Error>` - 在线成员分页结果
    async fn get_online_members_paged(&self, group_id: &str, page_size: i32, cursor: Option<String>) -> Result<GroupMemberPage>;

    /// 获取群状态
    /// 
    /// # 参数
    /// * `group_id` - 群ID
    /// 
    /// # 返回
    /// * `Result<GroupStatus, Error>` - 群状态
    async fn get_group_status(&self, group_id: &str) -> Result<GroupStatus>;

    /// 更新群最新消息
    /// 
    /// # 参数
    /// * `group_id` - 群ID
    /// * `message` - 最新消息
    /// 
    /// # 返回
    /// * `Result<(), Error>` - 更新成功返回Ok(()),失败返回具体错误
    async fn update_last_message(&self, group_id: &str, message: &Message) -> Result<()>;

    /// 增加群未读消息计数
    /// 
    /// # 参数
    /// * `group_id` - 群ID
    /// * `user_id` - 用户ID
    /// 
    /// # 返回
    /// * `Result<(), Error>` - 操作成功返回Ok(()),失败返回具体错误
    async fn increment_unread_count(&self, group_id: &str, user_id: &str) -> Result<()>;

    /// 解散群
    /// 
    /// # 参数
    /// * `group_id` - 群ID
    /// 
    /// # 返回
    /// * `Result<(), Error>` - 操作成功返回Ok(()),失败返回具体错误
    async fn dissolve_group(&self, group_id: &str) -> Result<()>;

    /// 获取群成员数量
    async fn get_member_count(&self, group_id: &str) -> Result<i64>;
}

// 模拟实现
pub struct MockGroupRepository;

#[async_trait]
impl GroupRepository for MockGroupRepository {
    async fn check_member_status(&self, _group_id: &str, _user_id: &str) -> Result<MemberStatus> {
        Ok(MemberStatus {
            is_member: true,
            is_muted: false,
            mute_expire_time: None,
            role: 0,
            join_time: None,
            last_speak_time: None,
            nickname: None,
        })
    }

    async fn get_group_members_paged(&self, _group_id: &str, query: GroupMemberQuery) -> Result<GroupMemberPage> {
        Ok(GroupMemberPage {
            members: vec!["user1".to_string(), "user2".to_string()],
            total: 2,
            cursor: None,
            has_more: false,
        })
    }

    async fn get_online_members_paged(&self, _group_id: &str, _page_size: i32, _cursor: Option<String>) -> Result<GroupMemberPage> {
        Ok(GroupMemberPage {
            members: vec!["user1".to_string()],
            total: 1,
            cursor: None,
            has_more: false,
        })
    }

    async fn get_group_status(&self, _group_id: &str) -> Result<GroupStatus> {
        Ok(GroupStatus {
            group_id: "group1".to_string(),
            name: "Group 1".to_string(),
            avatar: None,
            description: None,
            owner_id: "user1".to_string(),
            is_active: true,
            is_muted: false,
            mute_expire_time: None,
            create_time: 1633072800,
            update_time: 1633072800,
        })
    }

    async fn update_last_message(&self, _group_id: &str, _message: &Message) -> Result<()> {
        Ok(())
    }

    async fn increment_unread_count(&self, _group_id: &str, _user_id: &str) -> Result<()> {
        Ok(())
    }

    async fn dissolve_group(&self, _group_id: &str) -> Result<()> {
        Ok(())
    }

    async fn get_member_count(&self, _group_id: &str) -> Result<i64> {
        Ok(2)
    }
} 