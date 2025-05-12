#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::env;
    use std::path::Path;
    use chrono::Utc;
    use teloxide::types::{Chat, ChatId, ChatKind, ChatPrivate, MediaKind, MediaText, Message, MessageCommon, MessageId, MessageKind, User, UserId};
    use tokio::fs;
    use crate::{load_notes, is_valid_msg, add_note, note_from_message};
    use crate::config::CapturebotConfig;


    // Helper function to create a test message
    fn create_test_message(id: i32, text: &str, reply_to: Option<i32>) -> Message {
        let config = CapturebotConfig::for_testing("create_test_message");

        let user_id = config.user_id;
        
        Message {
            id: MessageId(id),
            thread_id: None,
            from: Some(User {
                id: UserId(user_id),
                first_name: "Test User".to_string(),
                last_name: None,
                username: None,
                is_bot: false,
                language_code: None,
                is_premium: false,
                added_to_attachment_menu: false,
            }),
            date: Utc::now(),
            chat: Chat {
                id: ChatId(0),
                kind: ChatKind::Private(ChatPrivate {
                    username: Some("capturebot".to_string()),
                    first_name: None,
                    last_name: None,
                }),
            },
            is_topic_message: false,
            via_bot: None,
            sender_chat: None,
            sender_business_bot: None,
            kind: MessageKind::Common(MessageCommon {
                media_kind: MediaKind::Text(MediaText { 
                    text: text.to_string(),
                    entities: Vec::new(),
                    link_preview_options: None
                }),
                reply_to_message: reply_to.map(|id| {
                    Box::new(Message {
                        id: MessageId(id),
                        thread_id: None,
                        from: Some(User {
                            id: UserId(user_id),
                            first_name: "Test User".to_string(),
                            last_name: None,
                            username: None,
                            is_bot: false,
                            language_code: None,
                            is_premium: false,
                            added_to_attachment_menu: false,
                        }),
                        date: Utc::now(),
                        chat: Chat {
                            id: ChatId(0),
                            kind: ChatKind::Private(ChatPrivate {
                                username: Some("capturebot".to_string()),
                                first_name: None,
                                last_name: None,
                            }),
                        },
                        is_topic_message: false,
                        via_bot: None,
                        sender_chat: None,
                        sender_business_bot: None,
                        kind: MessageKind::Common(MessageCommon {
                            media_kind: MediaKind::Text(MediaText {
                                text: "Parent message".to_string(),
                                entities: Vec::new(),
                                link_preview_options: None
                            }),
                            author_signature: None,
                            effect_id: None,
                            forward_origin: None,
                            reply_to_message: None,
                            external_reply: None,
                            quote: None,
                            reply_to_story: None,
                            sender_boost_count: None,
                            edit_date: None,
                            reply_markup: None,
                            is_automatic_forward: false,
                            has_protected_content: false,
                            is_from_offline: false,
                            business_connection_id: None                            
                        }),
                    })
                }),
                author_signature: None,
                effect_id: None,
                forward_origin: None,
                external_reply: None,
                quote: None,
                reply_to_story: None,
                sender_boost_count: None,
                edit_date: None,
                reply_markup: None,
                is_automatic_forward: false,
                has_protected_content: false,
                is_from_offline: false,
                business_connection_id: None,
            }),
        }
    }

    #[test]
    fn test_is_valid_msg() {
        let config = CapturebotConfig::for_testing("test_is_valid_msg");
        
        let valid_msg = create_test_message(1, "Test message", None);
        assert!(is_valid_msg(valid_msg, &config), "Message should be valid");
        
        // Create a message with different user ID
        let mut invalid_user_msg = create_test_message(2, "Test message", None);
        if let Some(user) = invalid_user_msg.from.as_mut() {
            user.id = UserId(99999); // Different user ID
        }
        assert!(!is_valid_msg(invalid_user_msg, &config), "Message with wrong user ID should be invalid");
    }

    #[tokio::test]
    async fn test_note_from_message() {

        let config = CapturebotConfig::for_testing("test_note_from_message");

        // Create test message
        let msg = create_test_message(1, "Test Title\nTest body content", None);
        let notes = HashMap::new();
        
        // Generate note from message
        let note = note_from_message(msg.clone(), &notes, &config);
        
        // Verify note properties
        assert_eq!(note.title, "Test Title");
        assert!(note.body.contains("Test Title"));
        assert!(note.body.contains("Test body content"));
        assert_eq!(note.capturebot_id, msg.id.to_string());
        assert!(note.path.to_str().unwrap().contains(".org"));
    }

    #[tokio::test]
    async fn test_load_notes() -> Result<(), std::io::Error> {
        // Set up test environment
        let test_config = CapturebotConfig::for_testing("test_load_notes");
        fs::create_dir_all(test_config.save_dir.as_path()).await.expect("create_dir_all failed");
        
        // Create a test note file
        let test_file_path_owned = test_config.save_dir.join("20230101000000-test-note.org");
        let test_content = format!(
            ":PROPERTIES:\n:ID: test-uuid\n:CREATED: [2023-01-01 Sun 00:00]\n:{}: 12345\n:END:\n#+title: Test Note\nTest content\n",
            crate::CAPTUREBOT_ID_PROPERTY
        );
        
        fs::write(test_file_path_owned.as_path(), test_content).await.expect("couldn't write test file");
        
        // Load notes
        let mut notes = HashMap::new();
        load_notes(&mut notes, &test_config).await.expect("load_notes failed");
        
        // Verify note was loaded
        assert!(notes.contains_key("12345"), "Note should be loaded with correct ID");
        let loaded_note = notes.get("12345").unwrap();
        assert_eq!(loaded_note.title, "Test Note");
        
        // Clean up test file
        fs::remove_file(test_file_path_owned.as_path()).await?;
        
        Ok(())
    }

    #[tokio::test]
    async fn test_add_note() -> Result<(), std::io::Error> {
        // Set up test environment
        let test_config = CapturebotConfig::for_testing("test_add_note");
        fs::create_dir_all(test_config.save_dir.as_path()).await?;
        
        // Create test message and notes map
        let msg = create_test_message(123, "Test Add Note\nThis is a test note", None);
        let mut notes = HashMap::new();
        
        // Add the note
        add_note(msg.clone(), &mut notes, &test_config).await?;
        
        // Verify note was added to the map
        assert!(notes.contains_key(&msg.id.to_string()), "Note should be added to the map");
        
        // Verify file was created
        let note = notes.get(&msg.id.to_string()).unwrap();
        assert!(Path::new(&note.path).exists(), "Note file should exist");
        
        // Clean up
        fs::remove_file(&note.path).await?;
        
        Ok(())
    }

    #[tokio::test]
    async fn test_reply_relationship() -> Result<(), std::io::Error> {
        // Set up test environment
        let test_config = CapturebotConfig::for_testing("test_reply_relationship");
        fs::create_dir_all(test_config.save_dir.as_path()).await?;
        
        // Create parent message and add it
        let parent_msg = create_test_message(456, "Parent Note\nThis is a parent note", None);
        let mut notes = HashMap::new();
        add_note(parent_msg.clone(), &mut notes, &test_config).await?;
        
        // Create a reply message and add it
        let reply_msg = create_test_message(789, "Reply Note\nThis is a reply note", Some(456));
        add_note(reply_msg.clone(), &mut notes, &test_config).await?;
        
        // Get the notes
        let parent_note = notes.get(&parent_msg.id.to_string()).unwrap();
        let reply_note = notes.get(&reply_msg.id.to_string()).unwrap();
        
        // Verify parent-child relationship
        assert!(reply_note.body.contains(&parent_note.id), "Reply should reference parent ID");
        
        // Clean up
        fs::remove_file(&parent_note.path).await?;
        fs::remove_file(&reply_note.path).await?;
        
        Ok(())
    }

    // Integration test for the whole flow
    #[tokio::test]
    async fn test_integration_flow() -> Result<(), std::io::Error> {
        // Set up test environment
        let test_config = CapturebotConfig::for_testing("test_integration_flow");
        fs::create_dir_all(test_config.save_dir.as_path()).await?;
        
        // Initialize empty notes map
        let mut notes = HashMap::new();
        
        // Create and add test messages
        let msg1 = create_test_message(1001, "First Note\nContent of first note", None);
        let msg2 = create_test_message(1002, "Second Note\nContent of second note", Some(1001));
        
        add_note(msg1.clone(), &mut notes, &test_config).await?;
        add_note(msg2.clone(), &mut notes, &test_config).await?;
        
        // Clear the notes map and reload from files
        notes.clear();
        load_notes(&mut notes, &test_config).await?;
        
        // Verify both notes were loaded correctly
        assert!(notes.contains_key(&msg1.id.to_string()), "First note should be loaded");
        assert!(notes.contains_key(&msg2.id.to_string()), "Second note should be loaded");
        
        // Verify relationship is maintained
        let note2 = notes.get(&msg2.id.to_string()).unwrap();
        let note1 = notes.get(&msg1.id.to_string()).unwrap();
        assert!(note2.body.contains(&note1.id), "Relationship should be maintained after reload");
        
        // Clean up
        for note in notes.values() {
            if Path::new(&note.path).exists() {
                fs::remove_file(&note.path).await?;
            }
        }
        
        Ok(())
    }
}
