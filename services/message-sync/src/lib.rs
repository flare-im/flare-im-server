pub mod message_sync {
    use tonic::{Request, Response, Status};
    use proto::api::im::message::message_sync_server::MessageSync;
    use proto::api::im::message::{
        SyncMessageRequest, SyncMessageResponse,
        GetSyncStateRequest, GetSyncStateResponse,
        UpdateSyncStateRequest, UpdateSyncStateResponse,
    };

    #[derive(Debug, Default)]
    pub struct MessageSyncService {}

    #[tonic::async_trait]
    impl MessageSync for MessageSyncService {
        async fn sync_message(
            &self,
            request: Request<SyncMessageRequest>,
        ) -> Result<Response<SyncMessageResponse>, Status> {
            todo!("Implement sync_message")
        }

        async fn get_sync_state(
            &self,
            request: Request<GetSyncStateRequest>,
        ) -> Result<Response<GetSyncStateResponse>, Status> {
            todo!("Implement get_sync_state")
        }

        async fn update_sync_state(
            &self,
            request: Request<UpdateSyncStateRequest>,
        ) -> Result<Response<UpdateSyncStateResponse>, Status> {
            todo!("Implement update_sync_state")
        }
    }
} 