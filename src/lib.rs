#![feature(iter_intersperse)]
use chrono::Utc;
use organic::parser::parse_file;
use organic::types::Document;
use slugify::slugify;
use std::collections::{self, HashMap};
use std::env;
use std::io::Error;
use std::path::{Path, PathBuf};
use teloxide::types::{Message, MessageEntityKind};
use tokio::fs;
use tokio::io::AsyncReadExt;
use uuidgen::gen_uuid;

static SAVEDIR: &str = "./out/";

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
        let properties_iterator = doc
            .get_additional_properties()
            .filter_map(|p| p.value.map(|v| (p.property_name, v.to_string())));
        let properties_map: HashMap<&str, String> =
            collections::HashMap::from_iter(properties_iterator);
        let cap_id = properties_map.get(CAPTUREBOT_ID_PROPERTY).unwrap();
        let note: CapturebotNote = CapturebotNote {
            id: properties_map.get("ID").unwrap().to_string(),
            path : doc.path.unwrap().to_path_buf(),
            capturebot_id: cap_id.to_string(),
            _capturebot_parent: properties_map
                .get(CAPTUREBOT_PARENT_ID_PROPERTY)
                .cloned(),
	    title: properties_map.get("title").unwrap().to_string(),
            body: doc.source.to_string()
        };
        Ok(note)
    }
}

fn note_from_message(msg: Message, notes: &HashMap<String, CapturebotNote>) -> CapturebotNote {
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
		format!("\n{CAPTUREBOT_PARENT_ID_PROPERTY}: {}", rt.id.to_string()));
    let org_parent_link_string = reply
	.map_or( String::new(), |rt| notes.get(&rt.id.to_string())
		 .map_or( String::new(), |pn|
			  format!("* Related: [[id:{}][{}]]\n", pn.id.to_string(), pn.title)));
    let target_path = format!(
        "{SAVEDIR}/{d}-{t}.org",
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



pub async fn load_notes(notes: &mut HashMap<String, CapturebotNote>) -> Result<(), Error> {
    let mut direntries = fs::read_dir(Path::new(SAVEDIR)).await.unwrap();
    while let Some(direntry) = direntries.next_entry().await.unwrap() {
        if direntry.metadata().await.unwrap().is_file()
            && direntry
                .file_name()
                .to_str()
                .is_some_and(|f| f.ends_with(".org"))
        {
            let mut f = fs::File::open(direntry.path().as_path()).await?;
            let mut s = String::new();
            f.read_to_string(&mut s).await?;
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

pub fn is_valid_msg(msg: Message) -> bool {
    let user_id = env::var("CAPTUREBOT_USER_ID")
        .expect("Specify the user ID capturebot should listen for as CAPTUREBOT_USER_ID environment variable")
        .parse::<u64>()
        .expect("User ID should be an integer");
    msg.text().is_some() && msg.clone().from.is_some_and(|u| u.id.0 == user_id)
}


pub async fn add_note(
    msg: Message,
    notes: &mut HashMap<String, CapturebotNote>,
) -> Result<(), Error> {
    let new_note = note_from_message(msg, notes);
    fs::write(Path::new(&new_note.path), new_note.body.clone())
        .await
        .and_then(|_| {
            notes.insert(new_note.capturebot_id.clone(), new_note);
            Ok(())
        })?;
    Ok(())
}
