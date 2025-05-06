use organic::parser::{parse, parse_file};
use organic::types::NodeProperty;
use std::fs;
use std::path::Path;
use teloxide::prelude::*;
use teloxide::types::MessageEntityKind;
use url::Url;
use uuidgen::gen_uuid;
use Utc::now;

static SAVEDIR: Path = Path::new("./out/");

static CAPTUREBOT_ID_PROPERTY: &str = "CAPTUREBOT_MESSAGE_ID";
static CAPTUREBOT_PARENT_ID_PROPERTY: &str = "CAPTUREBOT_PARENT_MESSAGE_ID";

struct CapturebotNote {
    id: String,
    path: Path,
    telegram_id: MessageId,
    telegram_parent: Option<MessageId>,
    document: Document<'_>,
}

fn load_notes() -> Vec<CapturebotNote> {
    let mut result = Vec::new();
    match fs::ls(SAVEDIR) {
        Err(why) => println!("! {:?}", why.kind()),
        Ok(paths) => {
            for path in paths {
                let mut f = File::open(path)?;
                let mut s = &str::new();
                f.read_to_string(&mut s);
                let doc = parse_file(s, path)?;
                let properties = doc.get_additional_properties();

                if let Some(m_id) =
                    properties.find(|p: &NodeProperty| p.property_name == CAPTUREBOT_ID_PROPERTY)
                {
                    result.push(CapturebotNote {
                        id: properties.find(|p: &NodeProperty| p.property_name == "ID"),
                        path,
                        telegram_id: m_id.value,
                        telegram_parent: if let Some(pm_id) =
                            (properties.find(|p: &NodeProperty| {
                                p.property_name == CAPTUREBOT_PARENT_ID_PROPERTY
                            })) {
                            pm_id.value
                        },
                        document: doc,
                    });
                }
            }
        }
    };
}

fn add_note(msg: Message, notes: &mut Vec<CapturebotNote>) -> io::Result<'_> {
    let entities = msg.parse_entities().unwrap_or_default().iter();
    let text = msg.text.unwrap();
    let title = text
        .split_whitespace()
        .next()
        .unwrap_or_else(format!("capturebot note at %s", Utc::now()));
    let links: &str = entities
        .filter_map(|m| match m.kind() {
            MessageEntityKind::TextLink => Some(m.text),
            MessageEntityKind::Url => Some(m.text),
            true => None,
        })
        .join(", ");
    let timestamp = msg.date.format("[%Y-%m-%d %a %H:%M]");
    let org_id = gen_uuid(true);
    let (parent_cap_id, parent_org_id_link_string, cap_parent_id_property_string) =
        if let Some(rt) = msg.reply_to_message {
            (
                rt.id,
                if let Some(n) = notes
                    .iter()
                    .find(|n| n.telegram_id == rt.id)
                    {format!("\nReply to: [[id:{n.id}]]\n")},
                format!("\n{capturebot_parent_id_property}: {rt.id}"),
            )
        };
    let target_path = format!("{SAVEDIR}/{d}-{t}.org", d = msg.date.format("%Y%m%d%H%M%S"), t = slugify!(title, max_length=5));
    let note_body = format!(
        "
:PROPERTIES:
ID: {org_id}
CREATED: {timestamp}
{capturebot_id_property}:{msg.id}{cap_parent_id_property_string}
ROAM_REFS: {links}
:END:
#+title: {title}
{text}
{parent_org_id_link_string}
");
    return fs::write(target_path, note_body).and_then({
        notes.push(
            CapturebotNote {
                id: org_id,
                path: target_path,
                telegram_id: msg.id,
                telegram_parent: msg.reply_to_message,
                document: parse(&note_body)
            });
        return notes;
    })
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info("Starting throw dice bot...");

    let bot = Bot::from_env();

    teloxide::repl(bot, |bot: Bot, msg::Message| async move {
        bot.send_dice(msg.chat.id).await?;
        Ok(())
    })
    .await;
}
