# 消息服务架构说明

## 技术栈概览

- **基础框架**: Rust + Tokio异步运行时
- **通信协议**: gRPC (tonic) + WebSocket
- **API网关**: Axum
- **消息队列**: Kafka
- **缓存**: Redis Cluster
- **存储**: TimescaleDB + MinIO
- **服务发现**: etcd
- **监控追踪**: Prometheus + Jaeger
- **日志**: ELK Stack

## 消息服务拆分

原消息服务已拆分为四个独立的微服务，每个服务都有其特定的职责和扩展特点。

### 1. 消息路由服务 (message-router)

**职责**:
- 消息的路由和分发
- 查询用户在线状态和路由表
- 处理消息的ACK确认
- 负载均衡
- 消息加密传输

**技术实现**:
```toml
[dependencies]
tokio = { workspace = true }
tonic = { workspace = true }
redis = { workspace = true }
tracing = { workspace = true }
common = { path = "../../common" }
```

**关键流程**:
1. 接收来自Message Gateway的消息
2. 查询Redis获取接收方路由信息
3. 进行消息路由决策
4. 转发消息到目标Message Gateway
5. 处理消息ACK确认
6. 维护消息投递状态

**扩展特点**:
- 根据在线用户数和消息吞吐量扩展
- 关注点在路由性能和低延迟
- 支持水平扩展

### 2. 消息存储服务 (message-store)

**职责**:
- 消息持久化处理
- 写入消息队列
- 存储到时序数据库
- 消息去重
- 数据压缩
- 分片存储

**技术实现**:
```toml
[dependencies]
tokio = { workspace = true }
tonic = { workspace = true }
kafka = { workspace = true }
timescaledb = { workspace = true }
common = { path = "../../common" }
```

**存储方案**:
1. 分级存储:
   - L0: Redis缓存最近消息 (TTL: 24小时)
   - L1: TimescaleDB存储活跃消息 (保留30天)
   - L2: MinIO对象存储归档历史消息
2. 分片策略:
   - 按时间和会话ID分片
   - 冷热数据分离 (阈值可配置)
   - 采用高效压缩算法 (LZ4/Snappy)

**扩展特点**:
- 根据消息量和存储容量需求扩展
- 关注点在存储性能和数据可靠性
- 支持存储节点动态扩展

### 3. 消息同步服务 (message-sync)

**职责**:
- 多端消息同步
- 序列号管理
- 消息状态同步
- 消息撤回处理
- 删除处理
- 离线消息同步

**技术实现**:
```toml
[dependencies]
tokio = { workspace = true }
tonic = { workspace = true }
redis = { workspace = true }
kafka = { workspace = true }
common = { path = "../../common" }
```

**关键流程**:
1. 维护设备消息序列号
2. 处理多端消息同步请求
3. 管理消息状态变更
4. 处理消息撤回和删除
5. 同步离线消息

**数据结构**:
```rust
struct MessageSync {
    msg_id: String,
    seq_id: u64,
    device_id: String,
    sync_type: SyncType,
    timestamp: i64,
}

enum SyncType {
    New,
    Update,
    Delete,
    Recall,
}
```

**扩展特点**:
- 根据多端设备数量扩展
- 关注点在数据一致性
- 支持分区扩展

### 4. 消息过滤服务 (message-filter)

**职责**:
- 内容审核
- 反垃圾处理
- 敏感词过滤
- 媒体审核
- 规则管理
- 过滤策略配置

**技术实现**:
```toml
[dependencies]
tokio = { workspace = true }
tonic = { workspace = true }
redis = { workspace = true }
common = { path = "../../common" }
```

**关键流程**:
1. 接收消息内容
2. 进行敏感词检测
3. 执行反垃圾规则
4. 媒体内容审核
5. 返回过滤结果

**规则配置**:
```json
{
  "sensitive_words": ["word1", "word2"],
  "spam_rules": [
    {
      "type": "frequency",
      "limit": 10,
      "window": 60
    }
  ],
  "media_rules": {
    "max_size": 10485760,
    "allowed_types": ["image/jpeg", "image/png"]
  }
}
```

**扩展特点**:
- 根据内容审核需求扩展
- 关注点在审核准确性和处理速度
- 支持规则动态更新

### 5. 会话服务 (session-service)

**职责**:
- 管理用户会话状态
- 维护在线状态
- 处理心跳包
- 管理设备登录
- 多端会话同步
- 会话过期处理

**技术实现**:
```toml
[dependencies]
tokio = { workspace = true }
tonic = { workspace = true }
redis = { workspace = true }
etcd-client = { workspace = true }
common = { path = "../../common" }
```

**数据结构**:
```rust
struct Session {
    user_id: String,
    device_id: String,
    device_type: DeviceType,
    connection_id: String,
    last_active: i64,
    status: SessionStatus,
}

enum DeviceType {
    Mobile,
    Desktop,
    Web,
    Pad,
}

enum SessionStatus {
    Online,
    Away,
    Offline,
}
```

**关键流程**:
1. 用户登录时创建会话
2. 定期更新心跳状态
3. 检测会话活跃度
4. 处理会话超时
5. 同步多端状态

**扩展特点**:
- 支持水平扩展
- 关注点在会话一致性
- 支持多机房部署

### 6. 通知服务 (notification-service)

**职责**:
- 离线消息推送
- 系统通知管理
- 推送渠道管理
- 通知模板管理
- 推送策略控制
- 通知状态跟踪

**技术实现**:
```toml
[dependencies]
tokio = { workspace = true }
tonic = { workspace = true }
redis = { workspace = true }
kafka = { workspace = true }
common = { path = "../../common" }
```

**数据结构**:
```rust
struct Notification {
    notification_id: String,
    user_id: String,
    title: String,
    content: String,
    push_type: PushType,
    channel: PushChannel,
    status: PushStatus,
    created_at: i64,
}

enum PushType {
    Message,
    System,
    Activity,
}

enum PushChannel {
    APNS,
    FCM,
    HMS,
    MiPush,
}
```

**推送配置**:
```json
{
  "push_rules": {
    "max_retry": 3,
    "retry_interval": 60,
    "quiet_hours": {
      "start": "23:00",
      "end": "07:00"
    },
    "batch_size": 100
  },
  "channels": {
    "apns": {
      "sandbox": false,
      "topic": "com.flare.im"
    },
    "fcm": {
      "project_id": "flare-im"
    }
  }
}
```

**扩展特点**:
- 支持多推送渠道
- 灵活的推送策略
- 高可靠性保证

### 7. 媒体服务 (media-service)

**职责**:
- 媒体文件上传
- 文件格式转换
- 图片压缩处理
- 视频转码
- 文件存储管理
- CDN分发

**技术实现**:
```toml
[dependencies]
tokio = { workspace = true }
tonic = { workspace = true }
minio = { workspace = true }
kafka = { workspace = true }
common = { path = "../../common" }
```

**数据结构**:
```rust
struct MediaFile {
    file_id: String,
    user_id: String,
    file_type: MediaType,
    file_size: u64,
    mime_type: String,
    storage_path: String,
    cdn_url: String,
    created_at: i64,
}

enum MediaType {
    Image,
    Video,
    Audio,
    File,
}
```

**处理配置**:
```json
{
  "upload_limits": {
    "image": {
      "max_size": 10485760,
      "formats": ["jpg", "png", "gif"]
    },
    "video": {
      "max_size": 104857600,
      "formats": ["mp4", "mov"]
    }
  },
  "process_rules": {
    "image": {
      "max_width": 1920,
      "quality": 85,
      "format": "jpg"
    },
    "video": {
      "codec": "h264",
      "bitrate": "1000k"
    }
  }
}
```

**扩展特点**:
- 支持分布式存储
- 弹性处理能力
- CDN加速分发

### 8. 搜索服务 (search-service)

**职责**:
- 消息全文检索
- 用户搜索
- 群组搜索
- 文件搜索
- 搜索建议
- 热门搜索

**技术实现**:
```toml
[dependencies]
tokio = { workspace = true }
tonic = { workspace = true }
elasticsearch = "8.5"
redis = { workspace = true }
common = { path = "../../common" }
```

**数据结构**:
```rust
struct SearchQuery {
    keyword: String,
    search_type: SearchType,
    filters: Vec<SearchFilter>,
    page: u32,
    page_size: u32,
}

enum SearchType {
    Message,
    User,
    Group,
    File,
}

struct SearchFilter {
    field: String,
    operator: FilterOperator,
    value: String,
}
```

**索引配置**:
```json
{
  "message_index": {
    "settings": {
      "number_of_shards": 5,
      "number_of_replicas": 1
    },
    "mappings": {
      "properties": {
        "content": {
          "type": "text",
          "analyzer": "ik_max_word"
        },
        "sender": {
          "type": "keyword"
        },
        "timestamp": {
          "type": "date"
        }
      }
    }
  }
}
```

**扩展特点**:
- 支持分布式搜索
- 实时索引更新
- 高性能查询

## 消息可靠性保证

1. **ACK确认机制**
   - 消息发送方收到接收方确认后才认为发送成功
   - 支持多级别的ACK确认
   ```rust
   enum AckLevel {
       Received,    // 服务端收到
       Stored,      // 已持久化
       Delivered,   // 已投递
       Read        // 已读取
   }
   ```

2. **消息重试机制**
   - 未收到ACK的消息进行指数退避重试
   - 设置最大重试次数和时间间隔
   ```rust
   struct RetryPolicy {
       max_attempts: u32,
       base_delay: Duration,
       max_delay: Duration,
       multiplier: f64,
   }
   ```

3. **幂等性处理**
   - 通过消息ID去重
   - 确保消息不会重复处理
   ```rust
   struct MessageId {
       timestamp: i64,
       sequence: u32,
       node_id: u16,
   }
   ```

4. **持久化保证**
   - 消息写入Kafka后再返回发送成功
   - 支持多副本备份
   - 定期数据一致性检查

5. **分布式事务**
   - 采用最终一致性
   - 确保消息状态同步
   - 事务补偿机制

## 消息流转流程

1. **发送流程**:
   ```
   Client -> Message Gateway -> Message Router Service
   -> Message Filter Service (内容审核)
   -> Message Store Service (持久化)
   -> Message Router Service (查找路由)
   -> Target Message Gateway -> Target Client
   ```

2. **离线消息处理**:
   ```
   Kafka -> Message Store Service 
   -> TimescaleDB (存储)
   -> Notification Service (触发推送)
   -> 离线客户端
   ```

3. **消息同步流程**:
   ```
   Client -> Message Gateway -> Message Sync Service
   -> Message Store Service (查询)
   -> Message Sync Service (序列化)
   -> Message Gateway -> Client
   ```

## 性能优化

1. **消息路由优化**
   - 路由表缓存 (Redis)
   - 连接池复用 (连接池配置)
   - 批量处理 (批次大小可配置)
   ```rust
   struct RouterConfig {
       route_cache_ttl: Duration,
       conn_pool_size: u32,
       batch_size: u32,
   }
   ```

2. **存储优化**
   - 分片存储 (时间+会话ID)
   - 冷热分离 (访问频率)
   - 数据压缩 (LZ4/Snappy)
   - 索引优化 (时间+用户ID)

3. **同步优化**
   - 增量同步 (序列号机制)
   - 批量同步 (批次大小可配置)
   - 状态压缩 (位图压缩)

4. **过滤优化**
   - 规则缓存 (Redis)
   - 并行处理 (线程池)
   - 结果缓存 (TTL可配置)

## 监控指标

1. **性能指标**
   - 消息处理延迟
   - 消息吞吐量
   - 服务响应时间
   - 缓存命中率

2. **可靠性指标**
   - 消息投递成功率
   - 消息重试率
   - 消息丢失率
   - 服务可用性

3. **资源指标**
   - CPU使用率
   - 内存使用率
   - 磁盘使用率
   - 网络带宽使用率

## 部署建议

1. **资源配置**
   - 路由服务: CPU密集型
   - 存储服务: IO密集型
   - 同步服务: 内存密集型
   - 过滤服务: CPU密集型

2. **扩展策略**
   - 按服务负载特征扩展
   - 支持动态扩缩容
   - 考虑地理位置分布
   - 注意数据一致性 