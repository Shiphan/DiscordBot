use std::{borrow::Cow, convert::identity, str::FromStr};

use chrono::{DateTime, Duration, Timelike, Utc};
use chrono_tz::Tz;
use icu::{calendar::Gregorian, locale::locale};
use icu_datetime::{
    fieldsets::enums::ZonedDateAndTimeFieldSet,
    pattern::{DateTimePattern, FixedCalendarDateTimeNames},
};
use itertools::Itertools;
use js_sys::wasm_bindgen::JsValue;
use serde::{Deserialize, Serialize};
use serenity::{
    all::{GenericChannelId, Message, MessageId},
    builder::EditMessage,
    small_fixed_array::FixedString,
};
use worker::{
    Delay, DurableObject, Env, Fetch, Headers, Method, Request, RequestInit, Response,
    RouteContext, State, durable_object, wasm_bindgen,
};
use writeable::TryWriteable;

pub async fn clock(
    time_zones: String,
    interaction_token: FixedString<u32>,
    route_context: RouteContext<()>,
    ctx: &worker::Context,
) -> String {
    let time_zones = match time_zones
        .split_whitespace()
        .into_iter()
        .map(|x| match Tz::from_str(x) {
            Ok(time_zone) => Ok(time_zone),
            Err(_) => Err(format!("`{x}`")),
        })
        .partition_result::<Vec<Tz>, Vec<String>, _, _>()
    {
        (time_zones, unknown_time_zones) if unknown_time_zones.is_empty() => time_zones,
        (_, unknown_time_zones) => {
            return format!(
                "Failed to parse those time zones: {} (see <https://en.wikipedia.org/wiki/List_of_tz_database_time_zones>)",
                unknown_time_zones.iter().join(", ")
            );
        }
    };

    let message = time_string(time_zones.iter()).unwrap_or_else(identity);

    ctx.wait_until(async move {
        let (channel_id, message_id) = match get_message_id(interaction_token, &route_context).await
        {
            Ok(x) => x,
            Err(e) => {
                worker::console_error!("Failed to get message id: {e}");
                return;
            }
        };
        if let Err(e) =
            start_durable_object(time_zones, channel_id, message_id, route_context).await
        {
            worker::console_error!("Failed to start durable object: {e}");
        }
    });

    message
}

async fn get_message_id(
    interaction_token: FixedString<u32>,
    route_context: &RouteContext<()>,
) -> worker::Result<(GenericChannelId, MessageId)> {
    let application_id = route_context.secret("DISCORD_APPLICATION_ID")?;

    // <https://docs.discord.com/developers/interactions/receiving-and-responding#get-original-interaction-response>
    let uri = format!(
        "https://discord.com/api/v10/webhooks/{application_id}/{interaction_token}/messages/@original"
    );

    let request = Request::new(&uri, Method::Get)?;

    let fetch = Fetch::Request(request);

    let mut attempt_count = 0;
    let mut response = loop {
        let mut response = fetch.send().await?;
        if response.status_code() == 200 {
            break response;
        }
        if attempt_count >= 10 {
            return Err(worker::Error::RustError(format!(
                "fetch discord api `{uri}` with status code is not 200, but {}, with body of `{:?}`",
                response.status_code(),
                response.text().await
            )));
        }
        attempt_count += 1;
        Delay::from(std::time::Duration::from_millis(500 * attempt_count.min(3))).await;
    };

    let Message { id, channel_id, .. } = response.json::<Message>().await?;
    Ok((channel_id, id))
}

async fn start_durable_object(
    time_zones: Vec<Tz>,
    channel_id: GenericChannelId,
    message_id: MessageId,
    route_context: RouteContext<()>,
) -> worker::Result<()> {
    let namespace = route_context.durable_object("WORLDCLOCK")?;
    let stub = namespace
        .id_from_name(&format!("{channel_id}/{message_id}"))?
        .get_stub()?;
    stub.fetch_with_request(Request::new_with_init(
        "http://domain/init",
        RequestInit::new()
            .with_method(Method::Put)
            .with_body(Some(JsValue::from_str(&serde_json::to_string(
                &ClockInfo {
                    time_zones,
                    channel_id,
                    message_id,
                },
            )?))),
    )?)
    .await?;

    Ok(())
}

async fn edit_message(
    new_message_content: &str,
    channel_id: &GenericChannelId,
    message_id: &MessageId,
    env: &Env,
) -> worker::Result<()> {
    let edit_json = serde_json::to_string(&EditMessage::new().content(new_message_content))?;
    let discord_token = env.secret("DISCORD_TOKEN")?;
    // <https://docs.discord.com/developers/resources/message#edit-message>
    let request = Request::new_with_init(
        &format!("https://discord.com/api/v10/channels/{channel_id}/messages/{message_id}"),
        &RequestInit::new()
            .with_method(Method::Patch)
            .with_headers(Headers::from_iter([
                ("Content-Type", "application/json"),
                ("Authorization", &format!("Bot {discord_token}")),
            ]))
            .with_body(Some(JsValue::from_str(&edit_json))),
    )?;
    Fetch::Request(request).send().await?;

    Ok(())
}

fn time_string<'a>(time_zones: impl Iterator<Item = &'a Tz>) -> Result<String, String> {
    let mut names = FixedCalendarDateTimeNames::<Gregorian, ZonedDateAndTimeFieldSet>::try_new(
        locale!("zh-TW").into(),
    )
    .map_err(|e| e.to_string())?;
    let pattern = "M月d日 bh:mm VVVV";
    let pattern = DateTimePattern::try_from_pattern_str(pattern).map_err(|e| e.to_string())?;
    names
        .include_for_pattern(&pattern)
        .map_err(|e| e.to_string())?;
    let formatter = names
        .include_for_pattern(&pattern)
        .map_err(|e| e.to_string())?;
    let utc_time = Utc::now();
    Itertools::intersperse(
        time_zones.map(|time_zone| {
            match formatter
                .format(&utc_time.with_timezone(time_zone))
                .try_write_to_string()
            {
                Ok(x) => Ok(Cow::Owned(x.into_owned())),
                Err((e, s)) => Err(format!("{e} ({s})")),
            }
        }),
        Ok(Cow::Borrowed(", ")),
    )
    .collect::<Result<String, String>>()

    // TODO: icu4x doesn't support `B`: <https://github.com/unicode-org/icu4x/issues/487>, <https://github.com/unicode-org/icu4x/pull/1216>
    /*
    let formatter = match DateTimeFormatter::try_new(
        locale!("zh-TW").into(),
        DT::long()
            .with_time_precision(TimePrecision::Minute)
            .with_zone(zone::Location),
    ) {
        Ok(x) => x,
        Err(e) => {
            return Err(format!("Failed to create date time formatter: {e}"));
        }
    };
    let utc_time = Utc::now();
    Ok(time_zones
        .map(|time_zone| formatter.format(&utc_time.with_timezone(time_zone)))
        .join(", "))
    */
}

#[durable_object]
pub struct Clock {
    state: State,
    env: Env,
}

#[derive(Deserialize, Serialize, Debug)]
struct ClockInfo {
    time_zones: Vec<Tz>,
    channel_id: GenericChannelId,
    message_id: MessageId,
}

impl DurableObject for Clock {
    fn new(state: State, env: Env) -> Self {
        Self { state, env }
    }
    async fn fetch(&self, mut req: Request) -> worker::Result<Response> {
        match req.path().as_str() {
            "/init" => {
                self.state
                    .storage()
                    .put("info", req.json::<ClockInfo>().await?)
                    .await?;
                self.state.storage().set_alarm(next_minute()).await?;
                Response::ok("ok")
            }
            "/delete" => {
                self.state.storage().delete_all().await?;
                self.state.storage().delete_alarm().await?;
                Response::ok("ok")
            }
            _ => Response::error("", 404),
        }
    }
    async fn alarm(&self) -> worker::Result<Response> {
        let Some(ClockInfo {
            time_zones,
            channel_id,
            message_id,
        }) = self.state.storage().get("info").await?
        else {
            worker::console_error!("called alarm function, but never initialize info");
            return Response::error("", 404);
        };

        let new_message = match time_string(time_zones.iter()) {
            Ok(x) => x,
            Err(e) => {
                return Err(worker::Error::RustError(format!(
                    "Failed to create new message: {e}"
                )));
            }
        };
        match edit_message(&new_message, &channel_id, &message_id, &self.env).await {
            Ok(()) => (),
            Err(e) => {
                let failed_count = self.state.storage().get("failed_count").await?.unwrap_or(0) + 1;
                if failed_count >= 10 {
                    worker::console_warn!(
                        "Failed to edit message for 10 times, going to delete this durable object"
                    );
                    self.state.storage().delete_all().await?;
                    self.state.storage().delete_alarm().await?;
                } else {
                    self.state
                        .storage()
                        .put("failed_count", failed_count)
                        .await?;
                }

                worker::console_error!("Failed to edit message: {e}");
            }
        }
        self.state.storage().set_alarm(next_minute()).await?;

        Response::ok("ok")
    }
}

fn next_minute() -> DateTime<Utc> {
    let now = Utc::now();
    now + (Duration::minutes(1)
        - Duration::seconds(now.second().into())
        - Duration::nanoseconds(now.nanosecond().into()))
}
