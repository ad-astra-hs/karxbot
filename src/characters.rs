use poise::serenity_prelude as serenity;

#[derive(Clone)]
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

#[derive(Clone)]
pub struct Character<'a> {
    pub name: &'a str,
    pub username: &'a str,
    pub alias: &'a str,
    pub replacements: Vec<(&'a str, &'a str)>,
    pub color: &'a str,
    pub image_url: &'a str,
    pub emoji: &'a str,
    pub case: Case,
}

impl Character<'_> {
    pub fn build_embed(self, text: String) -> serenity::CreateEmbed {
        let mut description = String::new();
        let mut in_asterisks = false;
        let text = text.trim();

        if text.starts_with('*') && text.ends_with('*') && text.matches('*').count() == 2 {
            description.push_str(text);
        } else {
            let mut parts = text.split('*').peekable();
            while let Some(part) = parts.next() {
                if in_asterisks {
                    description.push_str(&format!("*{}*", part));
                } else if part.trim().is_empty() {
                    continue;
                } else {
                    let processed_part = self.replacements.iter().fold(
                        self.case.apply(part),
                        |acc, (pattern, replace)| {
                            let re = regex::Regex::new(pattern).expect("Invalid regex pattern");
                            re.replace_all(&acc, *replace).into_owned()
                        },
                    );
                    description.push_str(&processed_part);
                }
                in_asterisks = !in_asterisks;
                if parts.peek().is_none() && in_asterisks {
                    description.push('*');
                }
            }
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

pub fn characters() -> Vec<Character<'static>> {
    vec![
        Character {
            name: "Malfaz Zoleum",
            username: "gogoGhost",
            alias: "gg",
            replacements: vec![
                ("^", "☾"),
                ("$", "☽"),
            ],
            color: "0x000000",
            image_url: "https://gitlab.com/ad-astra-hs/karxbot/-/raw/main/sprites/Malfaz%20Zoleum.png?ref_type=heads",
            emoji: "<:MalfazZoleum:1327671682936340591>",
            case: Case::Inverted,
        },
        Character {
            name: "Jozlyn Caluma",
            username: "phantasmataVisage",
            alias: "pv",
            replacements: vec![
                ("^", "⭂{"),
                ("$", "}⥵")
            ],
            color: "0xA10000",
            image_url: "https://gitlab.com/ad-astra-hs/karxbot/-/raw/main/sprites/Jozlyn%20Caluma.png?ref_type=heads",
            emoji: "<:JozlynCaluma:1327671634575757486>",
            case: Case::Title,
        },
        Character {
            name: "Febrez Galvan",
            username: "ancientAutomata",
            alias: "aa",
            replacements: vec![
                ("^|$" , " ⚙️ "),
                ("E", "3"),
                ("OO", "oOo"),
            ],
            color: "0xA15000",
            image_url: "https://gitlab.com/ad-astra-hs/karxbot/-/raw/main/sprites/Febrez%20Galvan.png?ref_type=heads",
            emoji: "<:FebrezGalvan:1327671624266420234>",
            case: Case::Upper,
        },
        Character {
            name: "Mixiek Kakkki",
            username: "sylphesterStallone",
            alias: "ss",
            replacements: vec![
                ("s|S", "$"),
                ("\"|,", ",,"),
                ("\\.|'", ","),
                ("(.+? .+?)( .+)", "$1,,$2"),
            ],
            color: "0xA1A100",
            image_url: "https://gitlab.com/ad-astra-hs/karxbot/-/raw/main/sprites/Mixiek%20Kakkki.png?ref_type=heads",
            emoji: "<:MixiekKakkki:1327671694365687888>",
            case: Case::Lower,
        },
        Character {
            name: "Eletra Zolage",
            username: "staticRebel",
            alias: "sr",
            replacements: vec![
                ("s", "z"),
                ("S", "Z"),
                ("l|L", "/"),
                ("c|C", "<")
            ],
            color: "0xE8E741",
            image_url: "https://gitlab.com/ad-astra-hs/karxbot/-/raw/main/sprites/Eletra%20Zolage.png?ref_type=heads",
            emoji: "<:EletraZolage:1327671614715855004>",
            case: Case::None,
        },
        Character {
            name: "Liaaam Galagr",
            username: "zombieBastards",
            alias: "zb",
            replacements: vec![
                ("[aeiouAEIOU]+", "$0 "),
            ],
            color: "0x626262",
            image_url: "https://gitlab.com/ad-astra-hs/karxbot/-/raw/main/sprites/Liaaam%20Galagr.png?ref_type=heads",
            emoji: "<:LiaaamGalagr:1327671671452074056>",
            case: Case::Lower,
        },
        Character {
            name: "Pennee Lechap",
            username: "tousleMimes",
            alias: "tm",
            replacements: vec![
                ("[a-zA-Z0-9]+", " ... ")
            ],
            color: "0x416600",
            image_url: "https://gitlab.com/ad-astra-hs/karxbot/-/raw/main/sprites/Pennee%20Lechap.png?ref_type=heads",
            emoji: "<:PenneeLechap:1327671717744611449>",
            case: Case::None,
        },
        Character {
            name: "Artyis Avelho",
            username: "freakinMagic",
            alias: "fm",
            replacements: vec![
                ("^", "[ "),
                ("$", " |###]")
            ],
            color: "0x008141",
            image_url: "https://gitlab.com/ad-astra-hs/karxbot/-/raw/main/sprites/Artyis%20Avelho.png?ref_type=heads",
            emoji: "<:ArtyisAvelho:1327671589424332962>",
            case: Case::None,
        },
        Character {
            name: "Lavena Perazi",
            username: "druryVigilante",
            alias: "dv",
            replacements: vec![
                ("^", "--- "),
                ("$", " ---O"),
                ("l", "L"),
            ],
            color: "0x008282",
            image_url: "https://gitlab.com/ad-astra-hs/karxbot/-/raw/main/sprites/Lavena%20Perazi.png?ref_type=heads",
            emoji: "<:LavenaPerazi:1327671662140981278>",
            case: Case::None,
        },
        Character {
            name: "Karxol Koomaa",
            username: "supernovaFruitcake",
            alias: "sf",
            replacements: vec![
                ("ck", "%"),
                ("k|%", "kk"),
                ("c", "k"),
                ("$", "."),
            ],
            color: "0x005682",
            image_url: "https://gitlab.com/ad-astra-hs/karxbot/-/raw/main/sprites/Karxol%20Koomaa.png?ref_type=heads",
            emoji: "<:KarxolKoomaa:1327671648861818992>",
            case: Case::Lower,
        },
        Character {
            name: "Tohbra Corrah",
            username: "screwedJobber",
            alias: "sj",
            replacements: vec![
                ("S", "%%%"),
                ("s", "£££"),
                ("Z", "S"),
                ("z", "s"),
                ("%%%", "Z"),
                ("£££", "z"),
                ("^", "<{"),
                ("$", "}<"),
            ],
            color: "0x000056",
            image_url: "https://gitlab.com/ad-astra-hs/karxbot/-/raw/main/sprites/Tohbra%20Corrah.png?ref_type=heads",
            emoji: "<:TohbraCorrah:1327671731900383242>",
            case: Case::Title,
        },
        Character {
            name: "Birsha Orobas",
            username: "solemnProphet",
            alias: "sp",
            replacements: vec![
                ("E|e", "Σ"),
                ("T|t", "✞"),
                ("$", "~"),
            ],
            color: "0x2B0057",
            image_url: "https://gitlab.com/ad-astra-hs/karxbot/-/raw/main/sprites/Birsha%20Orobas.png?ref_type=heads",
            emoji: "<:BirshaOrobas:1327671606289367090>",
            case: Case::None,
        },
        Character {
            name: "Tsoray Vodnik",
            username: "requiterElite",
            alias: "re",
            replacements: vec![
                ("\\.", ".~"),
                ("\\?", "?~"),
                ("\\!", "!~"),
                ("$", ".~"),
                ("A", "a"),
                ("E", "e"),
                ("I", "i"),
                ("O", "o"),
                ("U", "u"),
                ("Y", "y"),
                ("H", "h"),
            ],
            color: "0x6A006A",
            image_url: "https://gitlab.com/ad-astra-hs/karxbot/-/raw/main/sprites/Tsoray%20Vodnik.png?ref_type=heads",
            emoji: "<:TsorayVodnik:1327671743069949954>",
            case: Case::Upper,
        },
        Character {
            name: "Paiyuu Rowena",
            username: "abyssalPedestal",
            alias: "ap",
            replacements: vec![
                ("a|A", "☆")
            ],
            color: "0x77003C",
            image_url: "https://gitlab.com/ad-astra-hs/karxbot/-/raw/main/sprites/Paiyuu%20Rowena.png?ref_type=heads",
            emoji: "<:PaiyuuRowena:1327671705031803002>",
            case: Case::None,
        },
    ]
}
