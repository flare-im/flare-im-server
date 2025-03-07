fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .compile(
            &[
                "proto/common/message.proto",
                "proto/common/enums.proto",
                "proto/gateway/message.proto",
                "proto/message.proto",
                "proto/session.proto",
                "proto/notification.proto",
                "proto/media.proto",
                "proto/search.proto",
                "proto/user.proto",
                "proto/friend.proto",
            ],
            &["proto"],
        )?;
    Ok(())
} 