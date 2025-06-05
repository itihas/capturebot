use std::{collections::HashMap, fs::File, io::Read, path::PathBuf};

use capturebot::{add_note, is_valid_msg, load_notes, CapturebotConfig};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::from_str;
use teloxide::types::{Chat, Message, MessageId, MessageKind, User, UserId};

pub mod serde_user_id {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use teloxide::types::UserId;

    pub(crate) fn serialize<S>(this: &UserId, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        this.0.serialize(serializer)
    }

    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<UserId, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(UserId {
            0: String::deserialize(deserializer)?
                .strip_prefix("user")
                .unwrap()
                .parse::<u64>()
                .expect(""),
        })
    }
}

pub mod serde_date_from_unix_timestamp {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    fn serde_timestamp<E: serde::de::Error>(
        timestamp: i64,
    ) -> Result<chrono::DateTime<chrono::Utc>, E> {
        chrono::DateTime::from_timestamp(timestamp, 0).ok_or_else(|| E::custom("invalid timestump"))
    }

    pub(crate) fn serialize<S>(this: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        this.timestamp().serialize(serializer)
    }

    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse::<i64>()
            .map_err(serde::de::Error::custom)
            .and_then(serde_timestamp)
    }
}

#[serde_with::skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BackupMessage {
    pub id: i32,
    pub from: String,
    #[serde(with = "serde_user_id")]
    pub from_id: UserId,
    pub sender_chat: Option<Chat>,
    #[serde(rename = "date_unixtime", with = "serde_date_from_unix_timestamp")]
    pub date: DateTime<Utc>,
    #[serde(flatten)]
    pub kind: MessageKind,
}

impl From<BackupMessage> for Message {
    fn from(backup: BackupMessage) -> Self {
        Message {
            id: MessageId { 0: backup.id },
            thread_id: None,
            from: Some(User {
                id: backup.from_id,
                first_name: backup.from,
                last_name: None,
                username: None,
                is_bot: false,
                language_code: None,
                is_premium: false,
                added_to_attachment_menu: false,
            }),
            sender_chat: backup.sender_chat,
            date: backup.date,
            chat: Chat {
                id: teloxide::types::ChatId { 0: 0 },
                kind: teloxide::types::ChatKind::Private(teloxide::types::ChatPrivate {
                    username: Some("capturebot".to_string()),
                    first_name: None,
                    last_name: None,
                }),
            },
            is_topic_message: false,
            via_bot: None,
            sender_business_bot: None,
            kind: backup.kind,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct TelegramBackup {
    name: String,
    id: u64,
    messages: Vec<BackupMessage>,
}

#[tokio::main]
async fn main() {
    let config = CapturebotConfig::from_env();
    let backup_location = config.backup_json.clone().unwrap_or_else(|| {
        PathBuf::from(std::env::args().nth(1).expect("no backup location given"))
    }); // put this in config.rs later
    let mut file = File::open(backup_location).unwrap();
    let mut data = String::new();
    file.read_to_string(&mut data).unwrap();
    let json: TelegramBackup = from_str(&data).expect("backup file should be parseable as json");
    let mut notes = HashMap::new();
    load_notes(&mut notes, &config)
        .await
        .expect("load_notes failed");
    for backup_message in json.messages {
        let msg: Message = backup_message.clone().into();
        if is_valid_msg(msg.clone(), &config) {
            add_note(msg.clone(), &mut notes, &config)
                .await
                .map_err(|e| println!("parsing message {} failed: {}", msg.id.to_string(), e))
                .expect("");
        }
    }
}
