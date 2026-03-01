use serenity::{
    all::ComponentInteraction,
    builder::{CreateButton, CreateInteractionResponse, CreateInteractionResponseMessage},
    interactions_endpoint::Verifier,
    model::application::{CommandDataOptionValue, CommandInteraction, Interaction},
    small_fixed_array::FixedString,
};
use worker::{Env, Headers, Method, Request, Response, RouteContext, Router};

mod world_clock;
mod youtube_upload_timer;

#[worker::event(start)]
fn start() {
    console_error_panic_hook::set_once();
    worker::console_log!("A message from worker::event(start)!!!");
}

#[worker::event(fetch)]
pub async fn main(req: Request, env: Env, ctx: worker::Context) -> worker::Result<Response> {
    Router::new()
        .get("/hello", |_, _| Response::ok("Hello!!"))
        .post_async("/", |req, route_context| {
            bot_handler(req, route_context, &ctx)
        })
        .run(req, env)
        .await
}

async fn bot_handler(
    mut req: Request,
    route_context: RouteContext<()>,
    ctx: &worker::Context,
) -> worker::Result<Response> {
    // request to command and it's args
    let public_key = route_context.secret("DISCORD_PUBLIC_KEY")?.to_string();
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
        Interaction::Command(data) => handle_commands(data, route_context, ctx).await,
        Interaction::Component(data) => handle_component(data, route_context).await,
        _ => Response::error("Unknown Type", 400),
    };

    // worker::console_log!("body = {:?}, result = {result:?}", result.and_then(|x| x.cloned()).and_then(|x| futures::executor::block_on(x.text())));

    result
}

async fn handle_commands(
    command: CommandInteraction,
    route_context: RouteContext<()>,
    ctx: &worker::Context,
) -> worker::Result<Response> {
    worker::console_log!("application id = {}", command.application_id);
    let Ok(command) = Command::try_from(command) else {
        return Response::error("Unknown Type", 400);
    };

    // let discord_token = env.secret("DISCORD_TOKEN")?;
    match command {
        Command::UploadTimer {
            channel_id,
            search_keyword,
        } => {
            let youtube_api_key = route_context.secret("YOUTUBE_API_KEY")?.to_string();
            let response = youtube_upload_timer::upload_timer(
                channel_id.as_deref(),
                search_keyword.as_deref(),
                &youtube_api_key,
            )
            .await
            .unwrap_or_else(|e| format!("error while getting upload time from youtube api: {e}"));

            Response::from_json(&CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().content(response),
            ))
        }
        Command::Clock {
            time_zones,
            interaction_token,
        } => Response::from_json(&CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content(
                    world_clock::clock(time_zones, interaction_token, route_context, ctx).await,
                )
                .button(
                    CreateButton::new("world_clock/stop")
                        .label("Stop")
                        .emoji('\u{1F6D1}'),
                ),
        )),
    }
}

async fn handle_component(
    interaction: ComponentInteraction,
    route_context: RouteContext<()>,
) -> worker::Result<Response> {
    match interaction.data.custom_id.as_ref() {
        "world_clock/stop" => {
            let channel_id = &interaction.channel_id;
            let message_id = &interaction.message.id;
            let namespace = route_context.durable_object("WORLDCLOCK")?;
            let stub = namespace
                .id_from_name(&format!("{channel_id}/{message_id}"))?
                .get_stub()?;
            stub.fetch_with_request(Request::new("http://domain/delete", Method::Delete)?)
                .await?;
            Response::from_json(&CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new().components(&[]),
            ))
        }
        _ => Response::error("", 404),
    }
}

enum Command {
    UploadTimer {
        channel_id: Option<String>,
        search_keyword: Option<String>,
    },
    Clock {
        time_zones: String,
        interaction_token: FixedString<u32>,
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
            "clock" => value
                .data
                .options
                .iter()
                .find(|x| x.name == "timezones")
                .and_then(|x| x.value.as_str())
                .map(|x| Self::Clock {
                    time_zones: x.to_owned(),
                    interaction_token: value.token,
                })
                .ok_or(()),
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
