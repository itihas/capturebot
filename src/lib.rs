#![feature(iter_intersperse)]
mod config;
mod tests;

pub use crate::config::CapturebotConfig;
use chrono::Utc;
use organic::parser::parse_file;
use organic::types::{Document, Heading, StandardProperties};
use slugify::slugify;
use std::collections::{self, HashMap};
use std::io::Error;
use std::path::{Path, PathBuf};
use teloxide::types::{Message, MessageEntityKind};
use tokio::fs;
use uuidgen::gen_uuid;
use walkdir::WalkDir;

pub static CAPTUREBOT_ID_PROPERTY: &str = "CAPTUREBOT_MESSAGE_ID";
pub static CAPTUREBOT_PARENT_ID_PROPERTY: &str = "CAPTUREBOT_PARENT_MESSAGE_ID";

#[derive(Debug)]
pub struct CapturebotNote {
    pub id: String,
    pub path: PathBuf,
    pub capturebot_id: String,
    pub _capturebot_parent: Option<String>,
    pub title: String,
    pub body: String,
}

pub trait ContextualFrom<S, X, C>: Sized {
    type Error;
    fn contextual_from(value: S, context: X, config: C) -> Result<Self, Self::Error>;
}

impl ContextualFrom<&Document<'_>, &HashMap<String, CapturebotNote>, &CapturebotConfig>
    for CapturebotNote
{
    type Error = std::io::Error;
    fn contextual_from<'a>(
        doc: &Document,
        _notes: &HashMap<String, CapturebotNote>,
        _config: &CapturebotConfig,
    ) -> Result<Self, Self::Error> {
        let default_title = "untitled capturebot note".to_string();
        let title = doc
            .zeroth_section
            .iter()
            .flat_map(|zeroth_section| zeroth_section.children.iter())
            .find_map(|e| match e {
                organic::types::Element::Keyword(k) if k.key == "title" => Some(k.value),
                _ => None,
            })
            .unwrap_or(&default_title)
            .to_string();

        let properties_iterator = doc
            .get_additional_properties()
            .filter_map(|p| p.value.map(|v| (p.property_name, v.to_string())));
        let properties_map: HashMap<&str, String> =
            collections::HashMap::from_iter(properties_iterator);
        let cap_id = properties_map
            .get(CAPTUREBOT_ID_PROPERTY)
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "this note has no capturebot id, dropping",
            ))?;
        let note: CapturebotNote = CapturebotNote {
            id: properties_map
                .get("ID")
                .ok_or(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "this note has no id, dropping",
                ))?
                .to_string(),
            path: doc.path.clone().ok_or(Error::new(std::io::ErrorKind::InvalidData, "note should have a path"))?.to_path_buf(),
            capturebot_id: cap_id.to_string(),
            _capturebot_parent: properties_map.get(CAPTUREBOT_PARENT_ID_PROPERTY).cloned(),
            title,
            body: doc.source.to_string(),
        };
        Ok(note)
    }
}

impl ContextualFrom<&Heading<'_>, &HashMap<String, CapturebotNote>, &CapturebotConfig>
    for CapturebotNote
{
    type Error = std::io::Error;
    fn contextual_from<'a>(
        heading: &Heading,
        notes: &HashMap<String, CapturebotNote>,
        config: &CapturebotConfig,
    ) -> Result<Self, Self::Error> {
        let title = heading
            .title
            .iter()
            .map(|o| o.get_source())
            .collect::<String>();
        let properties_iterator = heading
            .get_additional_properties()
            .filter_map(|p| p.value.map(|v| (p.property_name, v.to_string())));
        let properties_map: HashMap<&str, String> =
            collections::HashMap::from_iter(properties_iterator);
        let cap_id = properties_map
            .get(CAPTUREBOT_ID_PROPERTY)
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "this note has no capturebot id, dropping",
            ))?;
        let note: CapturebotNote = CapturebotNote {
            id: properties_map
                .get("ID")
                .ok_or(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "this note has no id, dropping",
                ))?
                .to_string(),
            path: PathBuf::new(),
            capturebot_id: cap_id.to_string(),
            _capturebot_parent: properties_map.get(CAPTUREBOT_PARENT_ID_PROPERTY).cloned(),
            title,
            body: heading.get_source().to_string(),
        };
        Ok(note)
    }
}

impl ContextualFrom<Message, &HashMap<String, CapturebotNote>, &CapturebotConfig>
    for CapturebotNote
{
    type Error = std::io::Error;
    fn contextual_from(
        msg: Message,
        notes: &HashMap<String, CapturebotNote>,
        config: &CapturebotConfig,
    ) -> Result<CapturebotNote, Self::Error> {
        let text = msg.text().unwrap().to_string();
        let title = text.lines().next().map_or(
            format!("capturebot note made at {}", Utc::now()),
            str::to_string,
        );
        let links: String = msg
            .parse_entities()
            .unwrap_or_default()
            .iter()
            .filter_map(|m| match m.kind() {
                MessageEntityKind::TextLink { url } => Some(url.as_str()),
                MessageEntityKind::Url => Some(m.text()),
                _ => None,
            })
            .intersperse(", ")
            .collect();
        let timestamp = msg.date.format("[%Y-%m-%d %a %H:%M]");
        let org_id = gen_uuid(true);
        let cap_id = msg.id.to_string();
        let reply = msg.reply_to_message();
        let cap_parent_id_property_string = reply.map_or(String::new(), |rt| {
            format!("\n:{CAPTUREBOT_PARENT_ID_PROPERTY}: {}", rt.id.to_string())
        });
        let org_parent_link_string = reply.map_or(String::new(), |rt| {
            notes.get(&rt.id.to_string()).map_or(String::new(), |pn| {
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
            _capturebot_parent: msg.reply_to_message().map(|rt| rt.id.to_string()),
            title,
            body: note_body,
        })
    }
}

async fn load_from_dir(
    root_dir: PathBuf,
    notes: &mut HashMap<String, CapturebotNote>,
    config: &CapturebotConfig,
) -> Result<(), std::io::Error> {
    let direntries = WalkDir::new(root_dir.as_path());
    for direntry in direntries.into_iter().filter_map(|d| {
        if let Some(entry) = d.ok()
            && entry.file_type().is_file()
            && entry
                .file_name()
                .to_str()
                .is_some_and(|f| f.ends_with(".org"))
        {
            Some(entry)
        } else {
            None
        }
    }) {
        println!("{}", direntry.path().to_string_lossy());
        let s: String = tokio::fs::read_to_string(direntry.path())
            .await
            .inspect_err(|e| eprintln!("read_to_string failed: {:?}", e))?;
        let doc: Document<'_> = parse_file(&s, Some(direntry.path())).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("failed to parse contents of file {:?}: {:?}", direntry, e),
            )
        })?;
        if let Ok(note) = CapturebotNote::contextual_from(&doc, notes, config) {
            notes.entry(note.capturebot_id.to_string()).or_insert(note);
        } else {
            eprintln!("failed to create CapturebotNote for {:?}", direntry.path(),);
        }
        for heading in doc
            .children
            .iter()
        {
	    if let Ok(note) = CapturebotNote::contextual_from(heading, notes, config) {
		notes
                    .entry(note.capturebot_id.to_string())
                    .or_insert(note);
	    }
        }
    }
    Ok(())
}

pub async fn load_notes(
    mut notes: &mut HashMap<String, CapturebotNote>,
    config: &CapturebotConfig,
) -> Result<(), std::io::Error> {
    load_from_dir(config.read_dir.clone(), &mut notes, config).await?;
    load_from_dir(config.save_dir.clone(), &mut notes, config).await?;
    Ok(())
}

pub trait ValidMessage<C>: Sized {
    fn is_valid_msg(msg: Self, config: C) -> bool;
}

impl ValidMessage<&CapturebotConfig> for Message {
    fn is_valid_msg(msg: Self, config: &CapturebotConfig) -> bool {
        msg.text().is_some() && msg.clone().from.is_some_and(|u| u.id.0 == config.user_id)
    }
}

pub async fn add_note(
    msg: Message,
    notes: &mut HashMap<String, CapturebotNote>,
    config: &CapturebotConfig,
) -> Result<(), std::io::Error> {
    if notes.contains_key(&msg.id.to_string()) {
        println!("skipping {:?} : {:?}", msg.id, msg.text());
        Ok(())
    } else {
        println!("noting {:?} : {:?}", msg.id, msg.text());
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
