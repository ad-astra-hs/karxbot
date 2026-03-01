use std::sync::LazyLock;

use figment::{
    providers::{Format, Toml},
    Figment,
};
use poise::serenity_prelude as serenity;
use serde::Deserialize;

static QUOTE_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r#""([^"]*)""#).unwrap());
static ACTION_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"\*([^*]*)\*").unwrap());

#[derive(Deserialize)]
struct Config {
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
    #[serde(skip)]
    compiled_replacements: Vec<(regex::Regex, String)>,
    #[serde(skip)]
    compiled_color: u32,
}

impl Character {
    fn compile(mut self) -> Self {
        self.compiled_replacements = self
            .replacements
            .iter()
            .map(|r| {
                let re = regex::Regex::new(&r[0]).expect("Invalid regex in character config");
                (re, r[1].clone())
            })
            .collect();
        self.compiled_color = u32::from_str_radix(&self.color[2..], 16)
            .expect("Invalid hex color in character config");
        self
    }

    pub fn build_embed(self, text: String, paragraph: bool) -> serenity::CreateEmbed {
        let compiled_replacements = self.compiled_replacements;
        let case = self.case;
        let apply_transform = |s: &str| -> String {
            compiled_replacements
                .iter()
                .fold(case.apply(s), |acc, (re, replacement)| {
                    re.replace_all(&acc, replacement.as_str()).to_string()
                })
        };

        let description = if paragraph {
            QUOTE_RE
                .replace_all(&text, |caps: &regex::Captures| {
                    format!("\"{}\"", apply_transform(&caps[1]))
                })
                .to_string()
        } else {
            let mut last_end = 0;
            let mut result = String::new();
            for m in ACTION_RE.find_iter(&text) {
                result.push_str(&apply_transform(&text[last_end..m.start()]));
                result.push_str(m.as_str());
                last_end = m.end();
            }
            result.push_str(&apply_transform(&text[last_end..]));
            result
        };

        serenity::CreateEmbed::new()
            .title(self.username)
            .footer(serenity::CreateEmbedFooter::new(self.name))
            .colour(serenity::Colour(self.compiled_color))
            .thumbnail(self.image_url)
            .description(description)
    }
}

pub fn load_characters() -> Vec<Character> {
    let config_path = if std::path::Path::new("config.toml").exists() {
        "config.toml"
    } else {
        "example.config.toml"
    };

    let config = Figment::new()
        .merge(Toml::file(config_path))
        .extract::<Config>()
        .expect("Failed to load config. Please ensure config.toml or example.config.toml exists and is formatted correctly.");

    config.characters.into_iter().map(|c| c.compile()).collect()
}
