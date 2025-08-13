use std::env;

use reqwest::blocking::Response;
use serenity::{
    builder::{CreateCommand, CreateCommandOption},
    model::application::CommandOptionType,
};

fn main() {
    let application_id =
        env::var("DISCORD_APPLICATION_ID").expect("DISCORD_APPLICATION_ID is needed");
    let token = env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN is needed");

    let commands = [
        CreateCommand::new("uploadtimer")
            .description(
                "How long it has been since the last upload. (Use Guangyou\'s YouTube as default.)",
            )
            .set_options(vec![
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "channelid",
                    "YouTube channel ID",
                ),
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "search",
                    "The keyword to search for on Youtube",
                ),
            ]),
        CreateCommand::new("hi").description("this is a command that will (should) not response"),
    ];

    let response = register_commands(&commands, &application_id, &token);

    println!("response: {response:#?}");
    if let Ok(response) = response {
        println!("body: {:#?}", response.json::<serde_json::Value>());
    }
}

fn register_commands(
    commands: &[serenity::builder::CreateCommand],
    application_id: &str,
    token: &str,
) -> reqwest::Result<Response> {
    let url = format!("https://discord.com/api/v10/applications/{application_id}/commands");
    reqwest::blocking::Client::new()
        .put(url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bot {token}"))
        .json(commands)
        .send()
}
