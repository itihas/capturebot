#![feature(iter_intersperse)]
mod tests;
mod config;

use chrono::Utc;
pub use crate::config::CapturebotConfig;
use organic::parser::parse_file;
use organic::types::Document;
use slugify::slugify;
use std::collections::{self, HashMap};
use std::io::Error;
use std::path::{Path, PathBuf};
use teloxide::types::{Message, MessageEntityKind};
use tokio::fs;
use tokio::io::AsyncReadExt;
use uuidgen::gen_uuid;


static CAPTUREBOT_ID_PROPERTY: &str = "CAPTUREBOT_MESSAGE_ID";
static CAPTUREBOT_PARENT_ID_PROPERTY: &str = "CAPTUREBOT_PARENT_MESSAGE_ID";

pub struct CapturebotNote {
    id: String,
    path: PathBuf,
    capturebot_id: String,
    _capturebot_parent: Option<String>,
    title: String,
    body: String,
}

impl TryFrom<Document<'_>> for CapturebotNote {
    type Error = Error;
    fn try_from<'a>(doc: Document) -> Result<Self, Error> {
        let default_title = "untitled capturebot note".to_string();
        let title = doc.zeroth_section
            .iter()
            .flat_map(|zeroth_section| zeroth_section.children.iter())
            .find_map(|e| match e {
                organic::types::Element::Keyword(k) if k.key == "title" => Some(k.value),
                _ => None
            })
            .unwrap_or(&default_title)
            .to_string();

        let properties_iterator = doc
            .get_additional_properties()
            .filter_map(|p| p.value.map(|v| (p.property_name, v.to_string())));
        let properties_map: HashMap<&str, String> =
            collections::HashMap::from_iter(properties_iterator);
        let cap_id = properties_map.get(CAPTUREBOT_ID_PROPERTY).expect(format!("no cap_id property. note:\n{}", doc.contents).as_str());
        let note: CapturebotNote = CapturebotNote {
            id: properties_map.get("ID").expect("this note has no id").to_string(),
            path : doc.path.expect("note should have a path").to_path_buf(),
            capturebot_id: cap_id.to_string(),
            _capturebot_parent: properties_map
                .get(CAPTUREBOT_PARENT_ID_PROPERTY)
                .cloned(),
	    title,
            body: doc.source.to_string()
        };
        Ok(note)
    }
}

fn note_from_message(msg: Message, notes: &HashMap<String, CapturebotNote>, config: &CapturebotConfig) -> CapturebotNote {
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
    let cap_parent_id_property_string = reply
        .map_or(String::new(), |rt| 
		format!("\n:{CAPTUREBOT_PARENT_ID_PROPERTY}: {}", rt.id.to_string()));
    let org_parent_link_string = reply
	.map_or( String::new(), |rt| notes.get(&rt.id.to_string())
		 .map_or( String::new(), |pn|
			  format!("* Related: [[id:{}][{}]]\n", pn.id.to_string(), pn.title)));
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
    CapturebotNote {
        id: org_id,
        path: PathBuf::from(target_path),
        capturebot_id: msg.id.to_string(),
        _capturebot_parent: msg.reply_to_message().map(|rt| rt.id.to_string()),
	title,
        body: note_body
    }
}



pub async fn load_notes(notes: &mut HashMap<String, CapturebotNote>, config: &CapturebotConfig) -> Result<(), Error> {
    let mut direntries = fs::read_dir(config.save_dir.as_path()).await.expect("read_dir failed");
    while let Some(direntry) = direntries.next_entry().await.expect("next_entry failed") {
        if direntry.file_type().await.expect("file should be openable").is_file()
            && direntry
                .file_name()
                .to_str()
                .is_some_and(|f| f.ends_with(".org"))
        {
            let mut f = fs::File::open(direntry.path().as_path()).await.expect("file not openable");
            let mut s = String::new();
            println!("{}", direntry.path().to_string_lossy());
            f.read_to_string(&mut s).await.expect("reading file failed");
            let doc: Document<'_> = parse_file(&s, Some(direntry.path()))
                .map_err(|e| e.downcast::<Error>())
                .expect("document should be parseable");
            if let Some(note) = CapturebotNote::try_from(doc).ok() {
                notes.entry(note.capturebot_id.to_string()).or_insert(note);
            }
        }
    }
    Ok(())
}

pub fn is_valid_msg(msg: Message, config: &CapturebotConfig) -> bool {
    msg.text().is_some() && msg.clone().from.is_some_and(|u| u.id.0 == config.user_id)
}


pub async fn add_note(
    msg: Message,
    notes: &mut HashMap<String, CapturebotNote>,
    config: &CapturebotConfig
) -> Result<(), Error> {
    let new_note = note_from_message(msg, notes, config);
    fs::write(Path::new(&new_note.path), new_note.body.clone())
        .await
        .and_then(|_| {
            notes.insert(new_note.capturebot_id.clone(), new_note);
            Ok(())
        })?;
    Ok(())
}
