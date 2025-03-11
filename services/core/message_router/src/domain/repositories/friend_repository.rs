use async_trait::async_trait;
use anyhow::Result;
use crate::entities::Friendship;

/// 好友仓储接口
#[async_trait]
pub trait FriendRepository: Send + Sync {
    /// 检查好友关系
    /// 
    /// # 参数
    /// * `user_id` - 用户ID
    /// * `friend_id` - 好友ID
    /// 
    /// # 返回
    /// * `Result<Friendship, Error>` - 好友关系状态
    async fn check_friendship(&self, user_id: &str, friend_id: &str) -> Result<Friendship>;
}

// 模拟实现
pub struct MockFriendRepository;

#[async_trait]
impl FriendRepository for MockFriendRepository {
    async fn check_friendship(&self, _user_id: &str, _friend_id: &str) -> Result<Friendship> {
        Ok(Friendship {
            is_friend: true,
            in_blacklist: false,
            status: 2,
            remark: None,
            group: None,
            source: None,
            add_time: None,
            update_time: None,
        })
    }
} 