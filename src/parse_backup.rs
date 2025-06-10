#![feature(iter_intersperse)]
use std::{
    collections::HashMap,
    fs::File,
    io::{Error, Read},
    path::{Path, PathBuf},
};

use capturebot::{
    load_notes, CapturebotConfig, CapturebotNote, ContextualFrom, ValidMessage,
    CAPTUREBOT_ID_PROPERTY, CAPTUREBOT_PARENT_ID_PROPERTY,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::from_str;
use slugify::slugify;
use teloxide::types::{Chat, User};
use tokio::fs;
use uuidgen::gen_uuid;

pub mod serde_user_id {
    use serde::{Deserialize, Deserializer};

    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<u64, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(String::deserialize(deserializer)?
            .strip_prefix("user")
            .unwrap()
            .parse::<u64>()
            .unwrap_or_default())
    }
}

pub mod serde_date_from_unix_timestamp {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    fn serde_timestamp<E: serde::de::Error>(
        timestamp: i64,
    ) -> Result<chrono::DateTime<chrono::Utc>, E> {
        chrono::DateTime::from_timestamp(timestamp, 0).ok_or_else(|| E::custom("invalid timestamp"))
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

// impl From<BackupEntityKind> for MessageEntityKind {
//     fn from(value: BackupEntityKind) -> Self {
//         match value {
//             BackupEntityKind::Mention => MessageEntityKind::Mention,
//             BackupEntityKind::Hashtag => MessageEntityKind::Hashtag,
//             BackupEntityKind::Cashtag => MessageEntityKind::Cashtag,
//             BackupEntityKind::BotCommand => MessageEntityKind::BotCommand,
//             BackupEntityKind::Url => MessageEntityKind::Url,
//             BackupEntityKind::Email => MessageEntityKind::Email,
//             BackupEntityKind::PhoneNumber => MessageEntityKind::PhoneNumber,
//             BackupEntityKind::Bold => MessageEntityKind::Bold,
//             BackupEntityKind::Blockquote => MessageEntityKind::Blockquote,
//             BackupEntityKind::ExpandableBlockquote => MessageEntityKind::ExpandableBlockquote,
//             BackupEntityKind::Italic => MessageEntityKind::Italic,
//             BackupEntityKind::Underline => MessageEntityKind::Underline,
//             BackupEntityKind::Strikethrough => MessageEntityKind::Strikethrough,
//             BackupEntityKind::Spoiler => MessageEntityKind::Spoiler,
//             BackupEntityKind::Code => MessageEntityKind::Code,
//             BackupEntityKind::Pre { language } => MessageEntityKind::Pre { language },
//             BackupEntityKind::TextLink { url } => MessageEntityKind::TextLink { url },
//             BackupEntityKind::TextMention { user } => MessageEntityKind::TextMention { user },
//             BackupEntityKind::CustomEmoji { custom_emoji_id } => {
//                 MessageEntityKind::CustomEmoji { custom_emoji_id }
//             }
//         }
//     }
// }

// impl From<BackupEntity> for MessageEntity {
//     fn from(value: BackupEntity) -> Self {
//         MessageEntity {
//             kind: value.kind.into(),
//             // no offsets being recorded, which means that message parsing using entites will not work.
//             offset: 0,
//             length: 0,
//         }
//     }
// }

// impl From<BackupMessage> for Message {
//     fn from(backup: BackupMessage) -> Self {
//         let chat = Chat {
//             id: teloxide::types::ChatId { 0: 0 },
//             kind: teloxide::types::ChatKind::Private(teloxide::types::ChatPrivate {
//                 username: Some("capturebot".to_string()),
//                 first_name: None,
//                 last_name: None,
//             }),
//         };
//         Message {
//             id: MessageId { 0: backup.id },
//             thread_id: None,
//             from: Some(User {
//                 id: backup.from_id,
//                 first_name: backup.from,
//                 last_name: None,
//                 username: None,
//                 is_bot: false,
//                 language_code: None,
//                 is_premium: false,
//                 added_to_attachment_menu: false,
//             }),
//             sender_chat: backup.sender_chat,
//             date: backup.date,
//             chat: chat.clone(),
//             is_topic_message: false,
//             via_bot: None,
//             sender_business_bot: None,
//             kind: MessageKind::Common(teloxide::types::MessageCommon {
//                 author_signature: None,
//                 effect_id: None,
//                 forward_origin: None,
//                 reply_to_message: backup.reply_to_message_id.map(|m| {
//                     Box::new(Message {
//                         id: MessageId(m),
//                         thread_id: None,
//                         from: None,
//                         sender_chat: None,
//                         date: backup.date,
//                         chat: chat.clone(),
//                         is_topic_message: false,
//                         via_bot: None,
//                         sender_business_bot: None,
//                         kind: MessageKind::Empty {},
//                     })
//                 }),
//                 external_reply: None,
//                 quote: None,
//                 reply_to_story: None,
//                 sender_boost_count: None,
//                 edit_date: None,
//                 media_kind: MediaKind::Text(teloxide::types::MediaText {
//                     text: backup.text.to_string(),
//                     entities: backup.entities.iter().map(|e| e.clone().into()).collect(),
//                     link_preview_options: None,
//                 }),
//                 reply_markup: None,
//                 is_automatic_forward: false,
//                 has_protected_content: false,
//                 is_from_offline: false,
//                 business_connection_id: None,
//             }),
//         }
//     }
// }

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
enum BackupText {
    String(String),
    Parts(Vec<TextPart>),
}

impl ToString for BackupText {
    fn to_string(&self) -> String {
        match self {
            Self::String(s) => s.to_string(),
            Self::Parts(parts) => parts
                .iter()
                .map(|p| match p {
                    TextPart::String(s) => s.as_str(),
                    TextPart::Entity(BackupEntity { kind: _, text }) => text.as_str(),
                })
                .collect::<String>(),
        }
    }
}

#[serde_with::skip_serializing_none]
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct BackupMessage {
    pub id: i32,
    pub from: String,
    #[serde(with = "serde_user_id")]
    pub from_id: u64,
    pub sender_chat: Option<Chat>,
    #[serde(rename = "date_unixtime", with = "serde_date_from_unix_timestamp")]
    pub date: DateTime<Utc>,
    pub text: BackupText,
    pub media_type: Option<String>,
    pub reply_to_message_id: Option<i32>,
    #[serde(rename = "text_entities")]
    pub entities: Vec<BackupEntity>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
enum TextPart {
    String(String),
    Entity(BackupEntity),
}

#[serde_with::skip_serializing_none]
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct BackupEntity {
    #[serde(flatten)]
    pub kind: BackupEntityKind,
    pub text: String,
}

#[serde_with::skip_serializing_none]
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum BackupEntityKind {
    Plain,
    Mention,
    Hashtag,
    Cashtag,
    BotCommand,
    #[serde(rename = "link")]
    Url,
    Email,
    #[serde(rename = "phone")]
    PhoneNumber,
    Bold,
    Blockquote,
    ExpandableBlockquote,
    Italic,
    Underline,
    Strikethrough,
    Spoiler,
    Code,
    Pre {
        language: Option<String>,
    },
    TextLink {
        href: reqwest::Url,
    },
    TextMention {
        user: User,
    },
    CustomEmoji {
        custom_emoji_id: String,
    },
}

#[derive(Deserialize)]
struct TelegramBackup {
    name: String,
    id: u64,
    messages: Vec<BackupMessage>,
}

impl ValidMessage<&CapturebotConfig> for BackupMessage {
    fn is_valid_msg(msg: Self, config: &CapturebotConfig) -> bool {
        !msg.text.to_string().is_empty() && msg.clone().from_id == config.user_id
    }
}

impl ContextualFrom<BackupMessage, &HashMap<String, CapturebotNote>, &CapturebotConfig>
    for CapturebotNote
{
    type Error = Error;
    fn contextual_from(
        msg: BackupMessage,
        notes: &HashMap<String, CapturebotNote>,
        config: &CapturebotConfig,
    ) -> Result<Self, Self::Error> {
        let text = msg.text.to_string();
        let title = text.lines().next().map_or(
            format!("capturebot note made at {}", Utc::now()),
            str::to_string,
        );
        let target_path = format!(
            "{s}/{d}-{t}.org",
            s = config.save_dir.display(),
            d = msg.date.format("%Y%m%d%H%M%S"),
            t = slugify!(&title, max_length = 30)
        );
        let links = msg
            .entities
            .iter()
            .filter_map(|e| match e.clone().kind {
                BackupEntityKind::Url => Some(e.text.clone()),
                BackupEntityKind::TextLink { href } => Some(format!("[[{}][{}]]", e.text, href)),
                _ => None,
            })
            .intersperse(", ".to_string())
            .collect::<String>();
        let timestamp = msg.date.format("[%Y-%m-%d %a %H:%M]");
        let org_id = gen_uuid(true);
        let cap_id = msg.id.to_string();
        let reply = msg.reply_to_message_id;
        let cap_parent_id_property_string = reply.map_or(String::new(), |rt| {
            format!("\n:{CAPTUREBOT_PARENT_ID_PROPERTY}: {}", rt.to_string())
        });
        let org_parent_link_string = reply.map_or(String::new(), |rt| {
            notes.get(&rt.to_string()).map_or(String::new(), |pn| {
                format!("* Related: [[id:{}][{}]]\n", pn.id.to_string(), pn.title)
            })
        });
        let target_path = format!(
            "{s}/{d}-{t}.org",
            s = config.save_dir.display(),
            d = msg.date.format("%Y%m%d%H%M%S"),
            t = slugify!(&title, max_length = 30)
        );
        let note_body = format!(
            ":PROPERTIES:
:ID: {org_id}
:CREATED: {timestamp}
:{CAPTUREBOT_ID_PROPERTY}: {cap_id}{cap_parent_id_property_string}
:ROAM_REFS: {links}
:END:
#+title: {title}
{text}
{org_parent_link_string}
"
        );

        Ok(CapturebotNote {
            id: org_id,
            path: PathBuf::from(target_path),
            capturebot_id: msg.id.to_string(),
            _capturebot_parent: msg.reply_to_message_id.map(|rt| rt.to_string()),
            title,
            body: note_body,
        })
    }
}

pub async fn add_backup_note(
    msg: BackupMessage,
    notes: &mut HashMap<String, CapturebotNote>,
    config: &CapturebotConfig,
) -> Result<(), Error> {
    if notes.contains_key(&msg.id.to_string()) {
        println!("skipping {:?} : {:?}", msg.id, msg.text);
        Ok(())
    } else {
        println!("noting {:?} : {:?}", msg.id, msg.text);
        let new_note = CapturebotNote::contextual_from(msg, notes, config)?;
        fs::write(Path::new(&new_note.path), new_note.body.clone())
            .await
            .and_then(|_| {
                notes.insert(new_note.capturebot_id.clone(), new_note);
                Ok(())
            })?;
        Ok(())
    }
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
    for msg in json.messages {
        if BackupMessage::is_valid_msg(msg.clone(), &config) {
            add_backup_note(msg.clone(), &mut notes, &config)
                .await
                .map_err(|e| println!("parsing message {} failed: {}", msg.id.to_string(), e))
                .expect("");
        } else {
            println!("invalid message {:?}: {:?}", msg.id, msg)
        }
    }
}
