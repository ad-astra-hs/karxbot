use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

const SESSION_TIMEOUT_SECONDS: u64 = 300; // 5 minutes

#[derive(Clone, Serialize, Deserialize)]
pub struct DialogueSession {
    pub user_id: u64,
    pub channel_id: u64,
    pub spoiler_open: String,
    pub spoiler_close: String,
    pub messages: Vec<String>,
    pub started_at: u64,
    pub last_activity: u64,
}

impl DialogueSession {
    pub fn new(user_id: u64, channel_id: u64, spoiler_open: String, spoiler_close: String) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Self {
            user_id,
            channel_id,
            spoiler_open,
            spoiler_close,
            messages: Vec::new(),
            started_at: now,
            last_activity: now,
        }
    }

    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now - self.last_activity > SESSION_TIMEOUT_SECONDS
    }
}

pub fn init_sessions_table(conn: &Connection) {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS dialogue_sessions (
            user_id INTEGER NOT NULL,
            channel_id INTEGER NOT NULL,
            spoiler_open TEXT NOT NULL DEFAULT 'Dialogue',
            spoiler_close TEXT NOT NULL DEFAULT 'Close Dialogue',
            messages TEXT NOT NULL DEFAULT '[]',
            started_at INTEGER NOT NULL,
            last_activity INTEGER NOT NULL,
            PRIMARY KEY (user_id, channel_id)
        );",
    )
    .expect("Failed to initialise dialogue_sessions table");
}

pub fn create_session(
    conn: &Connection,
    user_id: u64,
    channel_id: u64,
    spoiler_open: String,
    spoiler_close: String,
) -> Result<(), rusqlite::Error> {
    let session = DialogueSession::new(user_id, channel_id, spoiler_open, spoiler_close);
    let messages_json = serde_json::to_string(&session.messages).unwrap();

    conn.execute(
        "INSERT INTO dialogue_sessions (user_id, channel_id, spoiler_open, spoiler_close, messages, started_at, last_activity)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(user_id, channel_id) DO UPDATE SET
            spoiler_open = ?3,
            spoiler_close = ?4,
            messages = ?5,
            started_at = ?6,
            last_activity = ?7",
        rusqlite::params![
            user_id as i64,
            channel_id as i64,
            session.spoiler_open,
            session.spoiler_close,
            messages_json,
            session.started_at as i64,
            session.last_activity as i64,
        ],
    )?;
    Ok(())
}

pub fn get_session(conn: &Connection, user_id: u64, channel_id: u64) -> Option<DialogueSession> {
    conn.query_row(
        "SELECT user_id, channel_id, spoiler_open, spoiler_close, messages, started_at, last_activity
         FROM dialogue_sessions WHERE user_id = ?1 AND channel_id = ?2",
        rusqlite::params![user_id as i64, channel_id as i64],
        |row| {
            let messages_json: String = row.get(4)?;
            let messages: Vec<String> = serde_json::from_str(&messages_json).unwrap_or_default();
            Ok(DialogueSession {
                user_id: row.get::<_, i64>(0)? as u64,
                channel_id: row.get::<_, i64>(1)? as u64,
                spoiler_open: row.get(2)?,
                spoiler_close: row.get(3)?,
                messages,
                started_at: row.get::<_, i64>(5)? as u64,
                last_activity: row.get::<_, i64>(6)? as u64,
            })
        },
    )
    .ok()
}

pub fn add_message(
    conn: &Connection,
    user_id: u64,
    channel_id: u64,
    message: String,
) -> Result<(), rusqlite::Error> {
    if let Some(mut session) = get_session(conn, user_id, channel_id) {
        session.messages.push(message);
        let messages_json = serde_json::to_string(&session.messages).unwrap();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        conn.execute(
            "UPDATE dialogue_sessions SET messages = ?1, last_activity = ?2
             WHERE user_id = ?3 AND channel_id = ?4",
            rusqlite::params![messages_json, now as i64, user_id as i64, channel_id as i64],
        )?;
    }
    Ok(())
}

pub fn delete_session(
    conn: &Connection,
    user_id: u64,
    channel_id: u64,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "DELETE FROM dialogue_sessions WHERE user_id = ?1 AND channel_id = ?2",
        rusqlite::params![user_id as i64, channel_id as i64],
    )?;
    Ok(())
}

pub fn cleanup_expired_sessions(conn: &Connection) -> Result<usize, rusqlite::Error> {
    let cutoff = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        - SESSION_TIMEOUT_SECONDS;

    let count = conn.execute(
        "DELETE FROM dialogue_sessions WHERE last_activity < ?1",
        rusqlite::params![cutoff as i64],
    )?;
    Ok(count)
}

pub fn has_active_session(conn: &Connection, user_id: u64, channel_id: u64) -> bool {
    if let Some(session) = get_session(conn, user_id, channel_id) {
        !session.is_expired()
    } else {
        false
    }
}

pub fn get_all_messages(conn: &Connection, user_id: u64, channel_id: u64) -> Option<Vec<String>> {
    get_session(conn, user_id, channel_id).map(|s| s.messages)
}

pub fn get_spoiler_labels(
    conn: &Connection,
    user_id: u64,
    channel_id: u64,
) -> Option<(String, String)> {
    get_session(conn, user_id, channel_id).map(|s| (s.spoiler_open, s.spoiler_close))
}
