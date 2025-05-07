#![feature(iter_intersperse)]
#[macro_use] extern crate slugify;
use slugify::slugify;
use chrono::{DateTime, Utc};
use organic::parser::{parse, parse_file};
use organic::types::{Document, NodeProperty};
use std::path::Path;
use std::io::Error;
use teloxide::prelude::*;
use teloxide::types::{Message, MessageEntityKind};
use tokio::fs;
use tokio::io::{AsyncReadExt};
use url::Url;
use uuidgen::gen_uuid;


static SAVEDIR: &str = "./out/";

static CAPTUREBOT_ID_PROPERTY: &str = "CAPTUREBOT_MESSAGE_ID";
static CAPTUREBOT_PARENT_ID_PROPERTY: &str = "CAPTUREBOT_PARENT_MESSAGE_ID";

struct CapturebotNote {
    id: String,
    path: String,
    capturebot_id: String,
    capturebot_parent: Option<String>,
}

fn get_property_value<'a>(
    p_name: String,
    properties: &impl Iterator<Item = NodeProperty<'a>>,
) -> Option<String> {
    todo!();
    // let result = properties
    //     .filter(|p: &NodeProperty| p.property_name == p_name)
    //     .map(|p| p.value.expect("If property exists, it should have a value").to_string())
    //     .next()
    //     .clone();
    // return result;
}

fn get_note_by_capturebot_id(
    cap_id: String,
    notes: &Vec<CapturebotNote>,
) -> Option<CapturebotNote> {
    todo!();
        // return notes
        // .filter(|n: &CapturebotNote| n.capturebot_id == cap_id)
        // .map(|n| n.value.expect("If note exists, it should have a cap_id").to_string())
        // .next()
        // .clone();
        // return None;
    
}

async fn load_notes(notes: &mut Vec<CapturebotNote>) -> Result<(), Error> {
    todo!();
    // let direntries = fs::read_dir(Path::new(SAVEDIR)).await.unwrap();
    // while let Some(direntry) = direntries.next_entry().await.unwrap() {
    //     if direntry.metadata().await.unwrap().is_file()
    //         && direntry
    //             .file_name()
    //             .to_str()
    //             .is_some_and(|f| f.ends_with(".org"))
    //     {
    //         let f = fs::File::open(direntry.path().as_path()).await?;
    //         let mut s;
    //         f.read_to_string(&mut s);
    //         let doc: Document<'a> = parse_file(s.as_str(), None).map_err(|e| e.downcast());
    //         let properties = doc.get_additional_properties();

    //         if let Some(cap_id) = get_property_value(CAPTUREBOT_ID_PROPERTY.to_string(), &properties) {
    //             if get_note_by_capturebot_id(cap_id, notes).is_none() {
    //                 notes.push(CapturebotNote {
    //                     id: get_property_value("ID".to_string(), &properties).unwrap(),
    //                     path: direntry.path().to_str().unwrap().to_string(),
    //                     capturebot_id: cap_id,
    //                     capturebot_parent: get_property_value(
    //                         CAPTUREBOT_PARENT_ID_PROPERTY.to_string(),
    //                         &properties,
    //                     ),
    //                 });
    //             }
    //         }
    //     }
    // };
    //     Ok(())
    
}

async fn add_note(msg: Message, notes: &mut Vec<CapturebotNote>) -> Result<(), Error> {
    let text = msg.text().unwrap().to_string();
    let title = text.lines().next().map_or(format!("capturebot note made at {}", Utc::now()), str::to_string);
    let links: String = msg.parse_entities().unwrap_or_default().iter().filter_map(|m| match m.kind() {
            MessageEntityKind::TextLink{url} => Some(url.as_str()),
            MessageEntityKind::Url => Some(m.text()),
            _ => None,
        })
        .intersperse(", ")
        .collect();
    let timestamp = msg.date.format("[%Y-%m-%d %a %H:%M]");
    let org_id = gen_uuid(true);
    let cap_id = msg.id.to_string();
    let (parent_org_id_link_string, cap_parent_id_property_string) = msg.reply_to_message()
        .map_or((String::new(), String::new()),
            |rt| {
                ( get_note_by_capturebot_id(rt.id.to_string(), notes)
                    .map_or(String::default(),|n| format!("\nReply to: [[id:{}]]\n", n.id) ),
                    format!("\n{CAPTUREBOT_PARENT_ID_PROPERTY}: {}", rt.id),
                )
            });
    let target_path = format!(
        "{SAVEDIR}/{d}-{t}.org",
        d = msg.date.format("%Y%m%d%H%M%S"),
        t = slugify!(title.as_str(), max_length = 5)
    );
    let note_body = format!(
        ":PROPERTIES:
ID: {org_id}
CREATED: {timestamp}
{CAPTUREBOT_ID_PROPERTY}:{cap_id}{cap_parent_id_property_string}
ROAM_REFS: {links}
:END:
#+title: {title}
{text}
{parent_org_id_link_string}
");
    let new_note = CapturebotNote {
        id: org_id,
        path: target_path.clone(),
        capturebot_id: msg.id.to_string(),
        capturebot_parent: msg.reply_to_message().map(|rt| rt.id.to_string()),
    };
    return fs::write(Path::new(target_path.as_str()), note_body.clone()).await.and_then(|_| {
        notes.push(new_note);
        Ok(())
    });
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("Starting capturebot...");

    let mut notes = Vec::new();
    load_notes(&mut notes);
    let bot = Bot::from_env();

    teloxide::repl(bot, |bot: Bot, msg: Message| async move {
        bot.send_dice(msg.chat.id).await?;
        // add_note(msg, &mut notes).await;
        Ok(())
    })
    .await;
}
