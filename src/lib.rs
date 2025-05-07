#![feature(iter_intersperse)]
use chrono::Utc;
use organic::parser::parse;
use organic::types::Document;
use slugify::slugify;
use std::collections::{self, HashMap};
use std::io::Error;
use std::path::Path;
use teloxide::types::{Message, MessageEntityKind};
use tokio::fs;
use tokio::io::AsyncReadExt;
use uuidgen::gen_uuid;

static SAVEDIR: &str = "./out/";

static CAPTUREBOT_ID_PROPERTY: &str = "CAPTUREBOT_MESSAGE_ID";
static CAPTUREBOT_PARENT_ID_PROPERTY: &str = "CAPTUREBOT_PARENT_MESSAGE_ID";

pub struct CapturebotNote {
    id: String,
    _path: String,
    capturebot_id: String,
    _capturebot_parent: Option<String>,
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
            let doc: Document<'_> = parse(s.as_str())
                .map_err(|e| e.downcast::<Error>())
                .expect("document should be parseable");
            let properties_iterator = doc
                .get_additional_properties()
                .filter_map(|p| p.value.map(|v| (p.property_name, v.to_string())));
            let properties_map: HashMap<&str, String> =
                collections::HashMap::from_iter(properties_iterator);

            if let Some(cap_id) = properties_map.get(CAPTUREBOT_ID_PROPERTY) {
                if notes.get(cap_id).is_none() {
                    let next_note = CapturebotNote {
                        id: properties_map.get("ID").unwrap().to_string(),
                        _path: direntry.path().to_str().unwrap().to_string(),
                        capturebot_id: cap_id.to_string(),
                        _capturebot_parent: properties_map
                            .get(CAPTUREBOT_PARENT_ID_PROPERTY)
                            .cloned(),
                    };
                    notes.insert(cap_id.to_string(), next_note);
                }
            }
        }
    }
    Ok(())
}

pub async fn add_note(msg: Message, notes: &mut HashMap<String, CapturebotNote>) -> Result<(), Error> {
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
    let (parent_org_id_link_string, cap_parent_id_property_string) =
        msg.reply_to_message()
            .map_or((String::new(), String::new()), |rt| {
                (
                    notes
                        .get(&rt.id.to_string())
                        .map_or(String::default(), |n| {
                            format!("\nReply to: [[id:{}]]\n", n.id)
                        }),
                    format!("\n{CAPTUREBOT_PARENT_ID_PROPERTY}: {}", rt.id),
                )
            });
    let target_path = format!(
        "{SAVEDIR}/{d}-{t}.org",
        d = msg.date.format("%Y%m%d%H%M%S"),
        t = slugify!(title.as_str(), max_length = 30)
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
{parent_org_id_link_string}
"
    );
    let new_note = CapturebotNote {
        id: org_id,
        _path: target_path.clone(),
        capturebot_id: msg.id.to_string(),
        _capturebot_parent: msg.reply_to_message().map(|rt| rt.id.to_string()),
    };
    fs::write(Path::new(target_path.as_str()), note_body.clone())
        .await
        .and_then(|_| {
            notes.insert(new_note.capturebot_id.clone(), new_note);
            Ok(())
        })?;
    Ok(())
}
