mod characters;
mod dialogue;

use characters::{load_characters, Character};
use dialogue::{
    add_message, cleanup_expired_sessions, create_session, delete_session, get_all_messages,
    get_spoiler_labels, has_active_session, init_sessions_table,
};
use poise::serenity_prelude as serenity;
use regex::Regex;
use rusqlite::Connection;
use std::sync::Mutex;
use std::time::Duration;

struct Data {
    db: Mutex<Connection>,
    characters: Vec<Character>,
}

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

fn init_db(conn: &Connection) {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS user_settings (
            uid            INTEGER PRIMARY KEY,
            last_used      TEXT,
            paragraph_mode INTEGER NOT NULL DEFAULT 0
        );",
    )
    .expect("Failed to initialise user_settings table");
    
    init_sessions_table(conn);
}

fn store_last_used(conn: &Connection, uid: u64, alias: &str) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO user_settings (uid, last_used) VALUES (?1, ?2)
         ON CONFLICT(uid) DO UPDATE SET last_used = ?2",
        rusqlite::params![uid as i64, alias],
    )?;
    Ok(())
}

fn read_last_used(conn: &Connection, uid: u64) -> Option<String> {
    conn.query_row(
        "SELECT last_used FROM user_settings WHERE uid = ?1",
        rusqlite::params![uid as i64],
        |row| row.get(0),
    )
    .ok()
    .flatten()
}

fn read_paragraph_mode(conn: &Connection, uid: u64) -> bool {
    conn.query_row(
        "SELECT paragraph_mode FROM user_settings WHERE uid = ?1",
        rusqlite::params![uid as i64],
        |row| row.get::<_, i64>(0),
    )
    .unwrap_or(0)
        != 0
}

fn toggle_paragraph_mode(conn: &Connection, uid: u64) -> Result<bool, rusqlite::Error> {
    let new_state = !read_paragraph_mode(conn, uid);
    conn.execute(
        "INSERT INTO user_settings (uid, paragraph_mode) VALUES (?1, ?2)
         ON CONFLICT(uid) DO UPDATE SET paragraph_mode = ?2",
        rusqlite::params![uid as i64, new_state as i64],
    )?;
    Ok(new_state)
}

/// Send a message as a character
#[poise::command(slash_command)]
async fn say(
    ctx: Context<'_>,
    #[description = "Character to send as"] alias: Option<String>,
    #[description = "Message to send"] text: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let (alias, paragraph) = {
        let db = ctx.data().db.lock().unwrap();
        if let Some(a) = &alias {
            store_last_used(&db, ctx.author().id.get(), a)?;
        }
        let alias =
            alias.unwrap_or_else(|| read_last_used(&db, ctx.author().id.get()).unwrap_or_default());
        let paragraph = read_paragraph_mode(&db, ctx.author().id.get());
        (alias, paragraph)
    };

    let character = ctx
        .data()
        .characters
        .iter()
        .find(|c| c.alias == alias)
        .ok_or("Character not found! Use `/list` for a list of available characters.")?;

    let embed = character.clone().build_embed(text, paragraph);
    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}

/// Toggle paragraph mode — when enabled, only text in quotes is quirked
#[poise::command(slash_command)]
async fn paragraph(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;
    let enabled = {
        let db = ctx.data().db.lock().unwrap();
        toggle_paragraph_mode(&db, ctx.author().id.get())?
    };
    let msg = if enabled {
        "Paragraph mode **enabled**. Only text within quotation marks will be quirked."
    } else {
        "Paragraph mode **disabled**. All text will be quirked."
    };
    ctx.say(msg).await?;
    Ok(())
}

/// List all available characters
#[poise::command(slash_command)]
async fn list(ctx: Context<'_>) -> Result<(), Error> {
    let embed = serenity::CreateEmbed::default().title("Characters").fields(
        ctx.data()
            .characters
            .iter()
            .map(|c| {
                (
                    format!("{} {}", c.emoji, c.name),
                    format!("Alias: `{}`", c.alias),
                    true,
                )
            })
            .collect::<Vec<_>>(),
    );
    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}

/// Start a dialogue session to collect MSPFA-formatted dialogue
#[poise::command(slash_command)]
async fn dialogue(
    ctx: Context<'_>,
    #[description = "Spoiler open label (default: Dialogue)"] open: Option<String>,
    #[description = "Spoiler close label (default: Close Dialogue)"] close: Option<String>,
) -> Result<(), Error> {
    let user_id = ctx.author().id.get();
    let channel_id = ctx.channel_id().get();
    
    let spoiler_open = open.unwrap_or_else(|| "Dialogue".to_string());
    let spoiler_close = close.unwrap_or_else(|| "Close Dialogue".to_string());
    
    {
        let db = ctx.data().db.lock().unwrap();
        create_session(
            &db, 
            user_id, 
            channel_id, 
            spoiler_open.clone(), 
            spoiler_close.clone()
        )?;
    }
    
    let components = vec![serenity::CreateActionRow::Buttons(vec![
        serenity::CreateButton::new("finish_dialogue")
            .label("Finish Dialogue")
            .style(serenity::ButtonStyle::Success),
        serenity::CreateButton::new("cancel_dialogue")
            .label("Cancel")
            .style(serenity::ButtonStyle::Danger),
    ])];
    
    let embed = serenity::CreateEmbed::new()
        .title("🎭 Dialogue Mode Active")
        .description(format!(
            "Type your dialogue lines in this channel. Format: `AB: Your dialogue here`
            
            **Spoiler Labels:** Open: `{}` | Close: `{}`
            
            Click **Finish Dialogue** when done, or **Cancel** to discard.
            
            *This session expires in 5 minutes.*",
            spoiler_open, spoiler_close
        ))
        .color(serenity::Colour::from_rgb(88, 101, 242));
    
    ctx.send(
        poise::CreateReply::default()
            .embed(embed)
            .components(components)
            .ephemeral(true),
    )
    .await?;
    
    Ok(())
}

fn parse_dialogue_line(line: &str) -> Result<(String, String), String> {
    if line.len() < 4 {
        return Err("Line too short".to_string());
    }
    
    let alias = line[0..2].to_ascii_lowercase();
    let text = line[4..].trim();
    
    if text.is_empty() {
        return Err("Empty dialogue text".to_string());
    }
    
    Ok((alias, text.to_string()))
}

fn apply_character_formatting(text: &str, character: &Character) -> String {
    let mut result = text.to_string();
    
    // Apply replacements using regex
    for replacement in &character.replacements {
        if replacement.len() >= 2 {
            if let Ok(re) = Regex::new(&replacement[0]) {
                result = re.replace_all(&result, replacement[1].as_str()).to_string();
            }
        }
    }
    
    // Apply case transformation
    character.case.apply(&result)
}

fn generate_mspfa_dialogue(
    messages: &[String],
    characters: &[Character],
    spoiler_open: &str,
    spoiler_close: &str,
) -> Result<String, String> {
    let mut dialogue_lines = Vec::new();
    
    for line in messages {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        
        let (alias, text) = match parse_dialogue_line(trimmed) {
            Ok(result) => result,
            Err(_) => continue,
        };
        
        let character = characters
            .iter()
            .find(|c| c.alias.to_ascii_lowercase() == alias)
            .ok_or_else(|| format!("Character with alias '{}' not found", alias))?;
        
        let formatted_text = apply_character_formatting(&text, character);
        
        // Ensure color starts with # for MSPFA format
        let color = if character.color.starts_with("0x") {
            character.color.replace("0x", "#")
        } else if !character.color.starts_with("#") {
            format!("#{}", character.color)
        } else {
            character.color.clone()
        };
        
        let line_output = format!(
            "[alt={}][color={}]{}: {}[/color][/alt]",
            text,
            color,
            alias.to_uppercase(),
            formatted_text
        );
        
        dialogue_lines.push(line_output);
    }
    
    if dialogue_lines.is_empty() {
        return Err("No valid dialogue lines found".to_string());
    }
    
    let spoiler_content = dialogue_lines.join("\n");
    let result = format!(
        r#"[spoiler open="{}" close="{}"]
{}
[/spoiler]"#,
        spoiler_open, spoiler_close, spoiler_content
    );
    
    Ok(result)
}

async fn handle_button_interaction(
    ctx: &serenity::Context,
    interaction: &serenity::ComponentInteraction,
    data: &Data,
) -> Result<(), Error> {
    let user_id = interaction.user.id.get();
    let channel_id = interaction.channel_id.get();
    
    match interaction.data.custom_id.as_str() {
        "finish_dialogue" => {
            // Check session and get data without holding lock across await
            let session_data = {
                let db = data.db.lock().unwrap();
                if !has_active_session(&db, user_id, channel_id) {
                    None
                } else {
                    let messages = get_all_messages(&db, user_id, channel_id);
                    let labels = get_spoiler_labels(&db, user_id, channel_id);
                    Some((messages, labels))
                }
            };
            
            let (messages, labels) = match session_data {
                Some((Some(msgs), Some(lbls))) => (msgs, lbls),
                _ => {
                    interaction
                        .create_response(
                            ctx,
                            serenity::CreateInteractionResponse::Message(
                                serenity::CreateInteractionResponseMessage::new()
                                    .content("❌ No active dialogue session found. Start one with `/dialogue`.")
                                    .ephemeral(true),
                            ),
                        )
                        .await?;
                    return Ok(());
                }
            };
            
            let (spoiler_open, spoiler_close) = labels;
            
            // Generate output
            match generate_mspfa_dialogue(&messages, &data.characters, &spoiler_open, &spoiler_close) {
                Ok(output) => {
                    // Delete session
                    {
                        let db = data.db.lock().unwrap();
                        delete_session(&db, user_id, channel_id)?;
                    }
                    
                    let code_block = format!("```\n{}\n```", output);
                    
                    interaction
                        .create_response(
                            ctx,
                            serenity::CreateInteractionResponse::Message(
                                serenity::CreateInteractionResponseMessage::new()
                                    .content(code_block),
                            ),
                        )
                        .await?;
                }
                Err(e) => {
                    interaction
                        .create_response(
                            ctx,
                            serenity::CreateInteractionResponse::Message(
                                serenity::CreateInteractionResponseMessage::new()
                                    .content(format!("❌ Error generating dialogue: {}", e))
                                    .ephemeral(true),
                            ),
                        )
                        .await?;
                }
            }
        }
        "cancel_dialogue" => {
            {
                let db = data.db.lock().unwrap();
                delete_session(&db, user_id, channel_id)?;
            }
            
            interaction
                .create_response(
                    ctx,
                    serenity::CreateInteractionResponse::Message(
                        serenity::CreateInteractionResponseMessage::new()
                            .content("✅ Dialogue session cancelled.")
                            .ephemeral(true),
                    ),
                )
                .await?;
        }
        _ => {}
    }
    
    Ok(())
}

async fn handle_message(
    _ctx: &serenity::Context,
    msg: &serenity::Message,
    data: &Data,
) -> Result<(), Error> {
    // Ignore bot messages
    if msg.author.bot {
        return Ok(());
    }
    
    let user_id = msg.author.id.get();
    let channel_id = msg.channel_id.get();
    
    let db = data.db.lock().unwrap();
    
    if !has_active_session(&db, user_id, channel_id) {
        return Ok(());
    }
    
    add_message(&db, user_id, channel_id, msg.content.clone())?;
    
    Ok(())
}

#[tokio::main]
async fn main() {
    let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");
    let intents = serenity::GatewayIntents::non_privileged()
        | serenity::GatewayIntents::MESSAGE_CONTENT
        | serenity::GatewayIntents::GUILD_MESSAGES;

    // Initialize database (setup will create the actual connection)
    {
        let conn = Connection::open("data/karxbot.db").expect("Failed to open database");
        init_db(&conn);
    }

    let characters = load_characters();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![say(), paragraph(), list(), dialogue()],
            event_handler: |ctx, event, _framework, data| {
                Box::pin(async move {
                    match event {
                        serenity::FullEvent::InteractionCreate { interaction, .. } => {
                            if let serenity::Interaction::Component(component) = interaction {
                                handle_button_interaction(ctx, component, data).await?;
                            }
                        }
                        serenity::FullEvent::Message { new_message } => {
                            handle_message(ctx, new_message, data).await?;
                        }
                        _ => {}
                    }
                    Ok(())
                })
            },
            ..Default::default()
        })
        .setup({
            let characters = characters.clone();
            move |ctx, _ready, framework| {
                Box::pin(async move {
                    poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                    
                    // Start cleanup task
                    tokio::spawn(async move {
                        let mut interval = tokio::time::interval(Duration::from_secs(60));
                        loop {
                            interval.tick().await;
                            if let Ok(conn) = Connection::open("data/karxbot.db") {
                                if let Ok(count) = cleanup_expired_sessions(&conn) {
                                    if count > 0 {
                                        println!("Cleaned up {} expired dialogue sessions", count);
                                    }
                                }
                            }
                        }
                    });
                    
                    let conn = Connection::open("data/karxbot.db")?;
                    Ok(Data { 
                        db: Mutex::new(conn), 
                        characters 
                    })
                })
            }
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;
    client.unwrap().start().await.unwrap();
}
