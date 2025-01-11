use poise::serenity_prelude as serenity;

struct Data {}
type Error=Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

fn characters() -> Vec<Character<'static>> {
    vec![
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
            image_url: "",
        },
        Character {
            name: "Karxol Koomaa",
            username: "supernovaFruitcake",
            alias: "sf",
            replacements: vec![
                ("ck", "%"),
                ("k|%", "kk"),
                ("$", "."),
            ],
            color: "0x005682",
            image_url: "",
        },
    ]
}

#[derive(Clone)]
struct Character<'a> {
    pub name: &'a str,
    pub username: &'a str,
    pub alias: &'a str,
    pub replacements: Vec<(&'a str,&'a str)>,
    pub color: &'a str,
    pub image_url: &'a str,
}

impl Character<'_> {
    fn build_embed(self, text: String) -> serenity::CreateEmbed {
        serenity::CreateEmbed::new()
            .title(self.username)
            .footer(serenity::CreateEmbedFooter::new(self.name))
            .colour(serenity::Colour(u32::from_str_radix(&self.color[2..], 16).expect("Invalid hex string")))
            .thumbnail(self.image_url)
            .description(self.replacements.iter().fold(text, |acc, (pattern, replace)| {
                let re = regex::Regex::new(pattern).expect("Invalid regex pattern");
                re.replace_all(&acc, *replace).into_owned()
            }))
    }
}

#[poise::command(slash_command)]
async fn say(
    ctx: Context<'_>,
    #[description = "Character to send as"] alias: String,
    #[description = "Message to send"] text: String,
) -> Result<(), Error> {
    let characters = characters();
    let character = characters.iter().find(|c| c.alias == alias).ok_or("Character not found")?;
    let embed = character.clone().build_embed(text);
    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}

#[tokio::main]
async fn main() {
    let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN");
    let intents = serenity::GatewayIntents::non_privileged();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![say()],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {})
            })
        })
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;
    client.unwrap().start().await.unwrap();
}