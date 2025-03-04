pub mod api {
    pub mod im {
        pub mod common {
            tonic::include_proto!("api.im.common");
        }
        
        pub mod message {
            tonic::include_proto!("api.im.message");
        }
        
        pub mod session {
            tonic::include_proto!("api.im.session");
        }
        
        pub mod notification {
            tonic::include_proto!("api.im.notification");
        }
        
        pub mod media {
            tonic::include_proto!("api.im.media");
        }
        
        pub mod search {
            tonic::include_proto!("api.im.search");
        }
        
        pub mod user {
            tonic::include_proto!("api.im.user");
        }
        
        pub mod friend {
            tonic::include_proto!("api.im.friend");
        }
    }
} 