mod characters;

use characters::characters;
use poise::serenity_prelude as serenity;

struct Data {}
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

#[poise::command(slash_command)]
async fn say(
    ctx: Context<'_>,
    #[description = "Character to send as"] alias: String,
    #[description = "Message to send"] text: String,
) -> Result<(), Error> {
    let characters = characters();
    let character = characters
        .iter()
        .find(|c| c.alias == alias)
        .ok_or("Character not found! Use `/list` for a list of available characters.")?;
    let embed = character.clone().build_embed(text);
    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}

#[poise::command(slash_command)]
async fn list(ctx: Context<'_>) -> Result<(), Error> {
    let characters = characters();
    let embed = serenity::CreateEmbed::default().title("Characters").fields(
        characters
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

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![say(), list()],
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
