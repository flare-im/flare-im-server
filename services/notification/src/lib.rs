pub mod notification {
    use tonic::{Request, Response, Status};
    use proto::api::im::notification::notification_server::Notification;
    use proto::api::im::notification::{
        SendNotificationRequest, SendNotificationResponse,
        GetNotificationsRequest, GetNotificationsResponse,
        MarkNotificationReadRequest, MarkNotificationReadResponse,
        DeleteNotificationRequest, DeleteNotificationResponse,
        GetUnreadCountRequest, GetUnreadCountResponse,
    };

    #[derive(Debug, Default)]
    pub struct NotificationService {}

    impl NotificationService {
        pub fn new() -> Self {
            Self {}
        }
    }

    #[tonic::async_trait]
    impl Notification for NotificationService {
        /// 发送通知
        async fn send_notification(
            &self,
            request: Request<SendNotificationRequest>,
        ) -> Result<Response<SendNotificationResponse>, Status> {
            todo!("Implement send_notification")
        }

        /// 获取通知列表
        async fn get_notifications(
            &self,
            request: Request<GetNotificationsRequest>,
        ) -> Result<Response<GetNotificationsResponse>, Status> {
            todo!("Implement get_notifications")
        }

        /// 标记通知为已读
        async fn mark_notification_read(
            &self,
            request: Request<MarkNotificationReadRequest>,
        ) -> Result<Response<MarkNotificationReadResponse>, Status> {
            todo!("Implement mark_notification_read")
        }

        /// 删除通知
        async fn delete_notification(
            &self,
            request: Request<DeleteNotificationRequest>,
        ) -> Result<Response<DeleteNotificationResponse>, Status> {
            todo!("Implement delete_notification")
        }

        /// 获取未读通知数量
        async fn get_unread_count(
            &self,
            request: Request<GetUnreadCountRequest>,
        ) -> Result<Response<GetUnreadCountResponse>, Status> {
            todo!("Implement get_unread_count")
        }
    }
}
