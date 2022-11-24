#[derive(Debug, thiserror::Error)]
pub enum RedisError {
    #[error("Failed to set key value in Redis")]
    SetFailed,
    #[error("Failed to set key value with expiry in Redis")]
    SetExFailed,
    #[error("Failed to set expiry for key value in Redis")]
    SetExpiryFailed,
    #[error("Failed to get key value in Redis")]
    GetFailed,
    #[error("Failed to delete key value in Redis")]
    DeleteFailed,
    #[error("Failed to append entry to redis stream")]
    StreamAppendFailed,
    #[error("Failed to read entries from redis stream")]
    StreamReadFailed,
    #[error("Failed to delete entries from redis stream")]
    StreamDeleteFailed,
    #[error("Failed to acknowledge redis stream entry")]
    StreamAcknowledgeFailed,
    #[error("Failed to create redis consumer group")]
    ConsumerGroupCreateFailed,
    #[error("Failed to destroy redis consumer group")]
    ConsumerGroupDestroyFailed,
    #[error("Failed to delete consumer from consumer group")]
    ConsumerGroupRemoveConsumerFailed,
    #[error("Failed to set last ID on consumer group")]
    ConsumerGroupSetIdFailed,
    #[error("Failed to set redis stream message owner")]
    ConsumerGroupClaimFailed,
    #[error("Failed to serialize application type to json")]
    JsonSerializationFailed,
    #[error("Failed to deserialize application type from json")]
    JsonDeserializationFailed,
}
