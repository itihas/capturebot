use capturebot::{add_note, load_notes, is_valid_msg};
use std::collections::HashMap;
use std::sync::Arc;
use teloxide::types::Message;
use teloxide::{RequestError, prelude::*};
use tokio::sync::Mutex;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("Starting capturebot...");

    let notes = Arc::new(Mutex::new(HashMap::new()));

    {
        let mut notes_guard = notes.lock().await;
        load_notes(&mut notes_guard)
            .await
            .expect("notes should all load before we can proceed");
    }

    let bot = Bot::from_env();

    teloxide::repl(bot, move |_bot: Bot, msg: Message| {
        let notes_clone = notes.clone();
        async move {
            // bot.send_dice(msg.chat.id).await?;
	    if is_valid_msg(msg) {
		let mut notes_guard = notes_clone.lock().await;
		add_note(msg, &mut notes_guard)
                    .await
                    .map_err(|e| RequestError::Io(e.into()))?;
	    }
            Ok(())
        }
    })
    .await;
}
