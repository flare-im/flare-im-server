pub mod api {
    pub mod im {
        pub mod common {
            tonic::include_proto!("api.im.common");
        }
        
        pub mod gateway {
            tonic::include_proto!("api.im.gateway");
        }

        pub mod service {
            pub mod store {
                tonic::include_proto!("api.im.service.store");
            }
            
            pub mod filter {
                tonic::include_proto!("api.im.service.filter");
            }

            pub mod router {
                tonic::include_proto!("api.im.service.router");
            }

            pub mod session {
                tonic::include_proto!("api.im.service.session");
            }

            pub mod sync {
                tonic::include_proto!("api.im.service.sync");
            }

            pub mod notification {
                tonic::include_proto!("api.im.service.notification");
            }
        }

        pub mod business {
            pub mod user {
                tonic::include_proto!("api.im.business.user");
            }
            
            pub mod group {
                tonic::include_proto!("api.im.business.group");
            }
            
            pub mod friend {
                tonic::include_proto!("api.im.business.friend");
            }
            
            pub mod media {
                tonic::include_proto!("api.im.business.media");
            }
            
            pub mod search {
                tonic::include_proto!("api.im.business.search");
            }
        }
    }
} 