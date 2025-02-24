use figment::{
    providers::{Format, Toml},
    Figment,
};
use poise::serenity_prelude as serenity;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    characters: Vec<Character>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Case {
    Lower,
    Upper,
    Title,
    Inverted,
    None,
}
impl Case {
    pub fn apply(&self, text: &str) -> String {
        match self {
            Case::Lower => text.to_lowercase(),
            Case::Upper => text.to_uppercase(),
            Case::Title => text
                .split_whitespace()
                .map(|word| {
                    let mut chars = word.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                    }
                })
                .collect::<Vec<String>>()
                .join(" "),
            Case::Inverted => text
                .chars()
                .map(|c| {
                    if c.is_lowercase() {
                        c.to_uppercase().next().unwrap()
                    } else {
                        c.to_lowercase().next().unwrap()
                    }
                })
                .collect(),
            Case::None => text.to_string(),
        }
    }
}

#[derive(Clone, Deserialize)]
pub struct Character {
    pub name: String,
    pub username: String,
    pub alias: String,
    pub replacements: Vec<Vec<String>>,
    pub color: String,
    pub image_url: String,
    pub emoji: String,
    pub case: Case,
}

impl Character {
    pub fn build_embed(self, text: String, action: bool) -> serenity::CreateEmbed {
        let mut description = String::new();
        if action {
            description.push_str(&format!("*{}*", text));
        } else {
            let processed_text =
                self.replacements
                    .iter()
                    .fold(self.case.apply(&text), |acc, replacement| {
                        let re = regex::Regex::new(&replacement[0]).expect("Invalid regex pattern");
                        re.replace_all(&acc, replacement[1].clone()).to_string()
                    });
            description.push_str(&processed_text);
        }

        serenity::CreateEmbed::new()
            .title(self.username)
            .footer(serenity::CreateEmbedFooter::new(self.name))
            .colour(serenity::Colour(
                u32::from_str_radix(&self.color[2..], 16).expect("Invalid hex string"),
            ))
            .thumbnail(self.image_url)
            .description(description)
    }
}

pub fn characters() -> Vec<Character> {
    let config = Figment::new()
        .merge(Toml::file("config.toml"))
        .extract::<Config>()
        .expect("Failed to load config.toml. Please ensure it exists and is formatted correctly.");

    config.characters
}
