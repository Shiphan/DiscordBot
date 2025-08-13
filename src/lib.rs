use serenity::{
    builder::{CreateInteractionResponse, CreateInteractionResponseMessage},
    interactions_endpoint::Verifier,
    model::application::{CommandDataOptionValue, CommandInteraction, Interaction},
};
use worker::{Env, Headers, Request, RequestInit, Response};

mod youtube_upload_timer;

#[worker::event(start)]
fn start() {
    worker::console_log!("A message from worker::event(start)!!!");
}

#[worker::event(fetch)]
pub async fn main(mut req: Request, env: Env, _ctx: worker::Context) -> worker::Result<Response> {
    let path = req.path();
    if path == "/hello" {
        return Response::ok("Hello!!");
    }
    if path != "/" {
        worker::console_log!("path == {}", req.path());
        return Response::error("Not found", 400);
    }

    // request to command and it's args
    let public_key = env.secret("DISCORD_PUBLIC_KEY")?.to_string();
    let verifier = Verifier::new(&public_key);
    let body = req.bytes().await?;
    match verify(req.headers(), &body, &verifier) {
        Ok(true) => (),
        Ok(false) => return Response::error("Unauthorized", 401),
        Err(e) => return worker::Result::Err(e),
    }
    let interaction: Interaction = serde_json::from_slice(&body)?;

    let result = match interaction {
        Interaction::Ping(_) => Response::from_json(&CreateInteractionResponse::Pong),
        Interaction::Command(data) => handle_commands(data, env).await,
        _ => Response::error("Unknown Type", 400),
    };

    worker::console_log!("{result:?}");

    result
}

async fn handle_commands(command: CommandInteraction, env: Env) -> worker::Result<Response> {
    let Ok(command) = Command::try_from(command) else {
        return Response::error("Unknown Type", 400);
    };

    // let discord_token = env.secret("DISCORD_TOKEN")?;
    match command {
        Command::UploadTimer {
            channel_id,
            search_keyword,
        } => {
            let youtube_api_key = env.secret("YOUTUBE_API_KEY")?.to_string();
            let response = youtube_upload_timer::upload_timer(
                channel_id.as_deref(),
                search_keyword.as_deref(),
                &youtube_api_key,
            )
            .await
            .map_err(|e| e.to_string())?;

            Response::from_json(&CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().content(response),
            ))
        }
    }
}

enum Command {
    UploadTimer {
        channel_id: Option<String>,
        search_keyword: Option<String>,
    },
}

impl TryFrom<CommandInteraction> for Command {
    type Error = ();

    fn try_from(value: CommandInteraction) -> Result<Self, Self::Error> {
        match value.data.name.to_lowercase().as_ref() {
            "uploadtimer" => {
                let options = value.data.options;
                let channel_id = options
                    .iter()
                    .find(|x| x.name == "channelid")
                    .and_then(|x| match &x.value {
                        CommandDataOptionValue::String(x) => Some(x.to_string()),
                        _ => None,
                    });
                let search_keyword =
                    options
                        .iter()
                        .find(|x| x.name == "search")
                        .and_then(|x| match &x.value {
                            CommandDataOptionValue::String(x) => Some(x.to_string()),
                            _ => None,
                        });
                Ok(Self::UploadTimer {
                    channel_id,
                    search_keyword,
                })
            }
            _ => Err(()),
        }
    }
}

fn verify(headers: &Headers, body: &[u8], verifier: &Verifier) -> worker::Result<bool> {
    let signature = headers
        .get("X-Signature-Ed25519")?
        .ok_or(worker::Error::RustError(
            "missing signature header".to_owned(),
        ))?;
    let timestamp = headers
        .get("X-Signature-Timestamp")?
        .ok_or(worker::Error::RustError(
            "missing timestamp header".to_owned(),
        ))?;
    Ok(verifier.verify(&signature, &timestamp, body).is_ok())
}
