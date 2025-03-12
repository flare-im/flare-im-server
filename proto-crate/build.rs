fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .compile_protos(
            &[
                "proto/common/error.proto",
                "proto/common/enums.proto",
                "proto/common/message.proto",
                "proto/common/server.proto",
                "proto/gateway/message.proto",
                "proto/gateway/api_gateway.proto",
                "proto/service/store.proto",
                "proto/service/filter.proto",
                "proto/service/router.proto",
                "proto/service/sync.proto",
                "proto/service/session.proto",
                "proto/service/notification.proto",
                "proto/business/user.proto",
                "proto/business/group.proto",
                "proto/business/friend.proto",
                "proto/business/media.proto",
                "proto/business/search.proto",
            ],
            &["proto"],
        )?;
    Ok(())
} 