mod characters;

use characters::{load_characters, Character};
use poise::serenity_prelude as serenity;
use rusqlite::Connection;
use std::sync::Mutex;

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
    .expect("Failed to initialise database");
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

#[tokio::main]
async fn main() {
    let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");
    let intents = serenity::GatewayIntents::non_privileged();

    let conn = Connection::open("data/karxbot.db").expect("Failed to open database");
    init_db(&conn);
    let db = Mutex::new(conn);

    let characters = load_characters();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![say(), paragraph(), list()],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data { db, characters })
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;
    client.unwrap().start().await.unwrap();
}
