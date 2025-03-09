use async_trait::async_trait;
use mongodb::{
    bson::{doc, Document, DateTime, Uuid as BsonUuid},
    Collection, Database,
    options::{ClientOptions, FindOptions, UpdateOptions},
};
use crate::domain::{
    entities::message::{Message, MessageQuery, MessageBatch, MessageStatus, MessageMetadata},
    repositories::message_repository::{MessageRepository, Error as RepoError},
};
use chrono::Utc;
use uuid::Uuid;
use futures::StreamExt;

pub struct MongoDBMessageRepository {
    collection: Collection<Document>,
}

impl MongoDBMessageRepository {
    pub async fn new(database_url: &str) -> Result<Self, RepoError> {
        let client_options = ClientOptions::parse(database_url)
            .await
            .map_err(|e| RepoError::Repository(e.to_string()))?;
        
        let client = mongodb::Client::with_options(client_options)
            .map_err(|e| RepoError::Repository(e.to_string()))?;
        
        let db = client.database("flare_im");
        let collection = db.collection("messages");

        // 创建索引
        collection.create_index(
            doc! {
                "session_id": 1,
                "created_at": -1
            },
            None
        ).await.map_err(|e| RepoError::Repository(e.to_string()))?;

        Ok(Self { collection })
    }

    // 将 Message 转换为 BSON Document
    fn to_document(message: &Message) -> Document {
        doc! {
            "_id": BsonUuid::from_uuid_v4(message.id),
            "session_id": &message.session_id,
            "sender_id": &message.sender_id,
            "content_type": &message.content_type,
            "content": &message.content,
            "status": match message.status {
                MessageStatus::Pending => "pending",
                MessageStatus::Sent => "sent",
                MessageStatus::Delivered => "delivered",
                MessageStatus::Read => "read",
                MessageStatus::Failed => "failed",
                MessageStatus::Deleted => "deleted",
            },
            "metadata": {
                "device_id": &message.metadata.device_id,
                "client_msg_id": &message.metadata.client_msg_id,
                "reply_to": message.metadata.reply_to.map(|id| BsonUuid::from_uuid_v4(id)),
                "mentions": &message.metadata.mentions,
                "is_encrypted": message.metadata.is_encrypted,
                "compression": &message.metadata.compression,
                "custom_properties": &message.metadata.custom_properties,
            },
            "created_at": DateTime::from_chrono(message.created_at),
            "updated_at": DateTime::from_chrono(message.updated_at),
        }
    }

    // 将 BSON Document 转换为 Message
    fn from_document(doc: Document) -> Result<Message, RepoError> {
        Ok(Message {
            id: doc.get_uuid("_id")
                .map_err(|e| RepoError::InvalidData(e.to_string()))?
                .into(),
            session_id: doc.get_str("session_id")
                .map_err(|e| RepoError::InvalidData(e.to_string()))?
                .to_string(),
            sender_id: doc.get_str("sender_id")
                .map_err(|e| RepoError::InvalidData(e.to_string()))?
                .to_string(),
            content_type: doc.get_str("content_type")
                .map_err(|e| RepoError::InvalidData(e.to_string()))?
                .to_string(),
            content: doc.get_str("content")
                .map_err(|e| RepoError::InvalidData(e.to_string()))?
                .to_string(),
            status: match doc.get_str("status").map_err(|e| RepoError::InvalidData(e.to_string()))? {
                "pending" => MessageStatus::Pending,
                "sent" => MessageStatus::Sent,
                "delivered" => MessageStatus::Delivered,
                "read" => MessageStatus::Read,
                "failed" => MessageStatus::Failed,
                "deleted" => MessageStatus::Deleted,
                _ => MessageStatus::Pending,
            },
            metadata: {
                let metadata = doc.get_document("metadata")
                    .map_err(|e| RepoError::InvalidData(e.to_string()))?;
                MessageMetadata {
                    device_id: metadata.get_str("device_id")
                        .map_err(|e| RepoError::InvalidData(e.to_string()))?
                        .to_string(),
                    client_msg_id: metadata.get_str("client_msg_id")
                        .map_err(|e| RepoError::InvalidData(e.to_string()))?
                        .to_string(),
                    reply_to: metadata.get_uuid("reply_to").ok().map(|id| id.into()),
                    mentions: metadata.get_array("mentions")
                        .map_err(|e| RepoError::InvalidData(e.to_string()))?
                        .iter()
                        .map(|v| v.as_str().unwrap_or_default().to_string())
                        .collect(),
                    is_encrypted: metadata.get_bool("is_encrypted")
                        .map_err(|e| RepoError::InvalidData(e.to_string()))?
                        .to_owned(),
                    compression: metadata.get_str("compression").ok().map(|s| s.to_string()),
                    custom_properties: metadata.get_document("custom_properties")
                        .map_err(|e| RepoError::InvalidData(e.to_string()))?
                        .iter()
                        .map(|(k, v)| (k.to_string(), v.as_str().unwrap_or_default().to_string()))
                        .collect(),
                }
            },
            created_at: doc.get_datetime("created_at")
                .map_err(|e| RepoError::InvalidData(e.to_string()))?
                .to_chrono(),
            updated_at: doc.get_datetime("updated_at")
                .map_err(|e| RepoError::InvalidData(e.to_string()))?
                .to_chrono(),
        })
    }
}

#[async_trait]
impl MessageRepository for MongoDBMessageRepository {
    async fn save(&self, message: Message) -> Result<(), RepoError> {
        let doc = Self::to_document(&message);
        self.collection.insert_one(doc, None)
            .await
            .map_err(|e| RepoError::Repository(e.to_string()))?;
        Ok(())
    }

    async fn get_by_id(&self, message_id: &str) -> Result<Option<Message>, RepoError> {
        let id = Uuid::parse_str(message_id)
            .map_err(|e| RepoError::InvalidData(e.to_string()))?;
        
        let filter = doc! { "_id": BsonUuid::from_uuid_v4(id) };
        
        if let Some(doc) = self.collection.find_one(filter, None)
            .await
            .map_err(|e| RepoError::Repository(e.to_string()))? {
            Ok(Some(Self::from_document(doc)?))
        } else {
            Ok(None)
        }
    }

    async fn delete(&self, message_id: &str) -> Result<(), RepoError> {
        let id = Uuid::parse_str(message_id)
            .map_err(|e| RepoError::InvalidData(e.to_string()))?;
        
        let filter = doc! { "_id": BsonUuid::from_uuid_v4(id) };
        
        self.collection.delete_one(filter, None)
            .await
            .map_err(|e| RepoError::Repository(e.to_string()))?;
        
        Ok(())
    }

    async fn batch_save(&self, messages: Vec<Message>) -> Result<(), RepoError> {
        let docs: Vec<Document> = messages.iter()
            .map(Self::to_document)
            .collect();
        
        self.collection.insert_many(docs, None)
            .await
            .map_err(|e| RepoError::Repository(e.to_string()))?;
        
        Ok(())
    }

    async fn batch_delete(&self, message_ids: Vec<String>) -> Result<(), RepoError> {
        let uuids: Result<Vec<BsonUuid>, _> = message_ids.iter()
            .map(|id| Uuid::parse_str(id).map(BsonUuid::from_uuid_v4))
            .collect();
        
        let filter = doc! { "_id": { "$in": uuids? } };
        
        self.collection.delete_many(filter, None)
            .await
            .map_err(|e| RepoError::Repository(e.to_string()))?;
        
        Ok(())
    }

    async fn query_messages(&self, query: MessageQuery) -> Result<MessageBatch, RepoError> {
        let mut filter = doc! { "session_id": &query.session_id };
        
        if let Some(start_time) = query.start_time {
            filter.insert("created_at", doc! { "$gte": DateTime::from_chrono(start_time) });
        }
        if let Some(end_time) = query.end_time {
            filter.insert("created_at", doc! { "$lte": DateTime::from_chrono(end_time) });
        }
        if let Some(content_types) = query.content_types {
            filter.insert("content_type", doc! { "$in": content_types });
        }
        if let Some(sender_id) = query.sender_id {
            filter.insert("sender_id", sender_id);
        }
        if let Some(status) = query.status {
            filter.insert("status", match status {
                MessageStatus::Pending => "pending",
                MessageStatus::Sent => "sent",
                MessageStatus::Delivered => "delivered",
                MessageStatus::Read => "read",
                MessageStatus::Failed => "failed",
                MessageStatus::Deleted => "deleted",
            });
        }

        let options = FindOptions::builder()
            .sort(doc! { "created_at": -1 })
            .skip(query.offset as u64)
            .limit(query.limit as i64)
            .build();

        let mut cursor = self.collection.find(filter.clone(), options)
            .await
            .map_err(|e| RepoError::Repository(e.to_string()))?;

        let mut messages = Vec::new();
        while let Some(result) = cursor.next().await {
            let doc = result.map_err(|e| RepoError::Repository(e.to_string()))?;
            messages.push(Self::from_document(doc)?);
        }

        let total_count = self.collection.count_documents(filter, None)
            .await
            .map_err(|e| RepoError::Repository(e.to_string()))?;

        Ok(MessageBatch {
            messages,
            total_count,
            has_more: total_count > (query.offset as u64 + query.limit as u64),
        })
    }

    async fn get_session_messages(&self, session_id: &str, limit: u32, offset: u32) -> Result<MessageBatch, RepoError> {
        let query = MessageQuery {
            session_id: session_id.to_string(),
            start_time: None,
            end_time: None,
            limit,
            offset,
            content_types: None,
            sender_id: None,
            status: None,
        };
        self.query_messages(query).await
    }

    async fn get_user_messages(&self, user_id: &str, limit: u32, offset: u32) -> Result<MessageBatch, RepoError> {
        let filter = doc! { "sender_id": user_id };
        
        let options = FindOptions::builder()
            .sort(doc! { "created_at": -1 })
            .skip(offset as u64)
            .limit(limit as i64)
            .build();

        let mut cursor = self.collection.find(filter.clone(), options)
            .await
            .map_err(|e| RepoError::Repository(e.to_string()))?;

        let mut messages = Vec::new();
        while let Some(result) = cursor.next().await {
            let doc = result.map_err(|e| RepoError::Repository(e.to_string()))?;
            messages.push(Self::from_document(doc)?);
        }

        let total_count = self.collection.count_documents(filter, None)
            .await
            .map_err(|e| RepoError::Repository(e.to_string()))?;

        Ok(MessageBatch {
            messages,
            total_count,
            has_more: total_count > (offset as u64 + limit as u64),
        })
    }

    async fn update_status(&self, message_id: &str, status: MessageStatus) -> Result<(), RepoError> {
        let id = Uuid::parse_str(message_id)
            .map_err(|e| RepoError::InvalidData(e.to_string()))?;
        
        let filter = doc! { "_id": BsonUuid::from_uuid_v4(id) };
        let update = doc! { 
            "$set": { 
                "status": match status {
                    MessageStatus::Pending => "pending",
                    MessageStatus::Sent => "sent",
                    MessageStatus::Delivered => "delivered",
                    MessageStatus::Read => "read",
                    MessageStatus::Failed => "failed",
                    MessageStatus::Deleted => "deleted",
                },
                "updated_at": DateTime::from_chrono(Utc::now())
            } 
        };
        
        self.collection.update_one(filter, update, None)
            .await
            .map_err(|e| RepoError::Repository(e.to_string()))?;
        
        Ok(())
    }

    async fn batch_update_status(&self, message_ids: Vec<String>, status: MessageStatus) -> Result<(), RepoError> {
        let uuids: Result<Vec<BsonUuid>, _> = message_ids.iter()
            .map(|id| Uuid::parse_str(id).map(BsonUuid::from_uuid_v4))
            .collect();
        
        let filter = doc! { "_id": { "$in": uuids? } };
        let update = doc! { 
            "$set": { 
                "status": match status {
                    MessageStatus::Pending => "pending",
                    MessageStatus::Sent => "sent",
                    MessageStatus::Delivered => "delivered",
                    MessageStatus::Read => "read",
                    MessageStatus::Failed => "failed",
                    MessageStatus::Deleted => "deleted",
                },
                "updated_at": DateTime::from_chrono(Utc::now())
            } 
        };
        
        self.collection.update_many(filter, update, None)
            .await
            .map_err(|e| RepoError::Repository(e.to_string()))?;
        
        Ok(())
    }

    async fn count_session_messages(&self, session_id: &str) -> Result<u64, RepoError> {
        let filter = doc! { "session_id": session_id };
        
        self.collection.count_documents(filter, None)
            .await
            .map_err(|e| RepoError::Repository(e.to_string()))
    }

    async fn count_user_messages(&self, user_id: &str) -> Result<u64, RepoError> {
        let filter = doc! { "sender_id": user_id };
        
        self.collection.count_documents(filter, None)
            .await
            .map_err(|e| RepoError::Repository(e.to_string()))
    }
} 