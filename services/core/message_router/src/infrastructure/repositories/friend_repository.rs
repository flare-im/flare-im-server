use async_trait::async_trait;
use anyhow::Result;
use crate::domain::repositories::FriendRepository;
use crate::entities::Friendship;

pub struct FriendRepositoryImpl;

impl FriendRepositoryImpl {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl FriendRepository for FriendRepositoryImpl {
    async fn check_friendship(&self, _user_id: &str, _friend_id: &str) -> Result<Friendship> {
        // 返回固定的友好关系数据
        Ok(Friendship {
            is_friend: true,
            in_blacklist: false,
            status: 2, // 2表示正常好友关系
            remark: Some("好友备注".to_string()),
            group: Some("我的好友".to_string()),
            source: Some("搜索添加".to_string()),
            add_time: Some(chrono::Utc::now().timestamp()),
            update_time: Some(chrono::Utc::now().timestamp()),
        })
    }
} 