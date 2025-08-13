use std::{borrow::Cow, error::Error};

use icu::{
    decimal::input::Decimal,
    experimental::relativetime::{
        RelativeTimeFormatter, RelativeTimeFormatterOptions, options::Numeric,
    },
    locale::locale,
};
use time::UtcDateTime;

pub async fn upload_timer(
    channel_id: Option<&str>,
    search_keyword: Option<&str>,
    youtube_api_key: &str,
) -> Result<String, Box<dyn Error>> {
    let channel_id = match channel_id {
        Some(x) => Cow::from(x),
        None => match search_keyword {
            Some(x) => Cow::from(get_channel_id_from_keyword(youtube_api_key, x).await?),
            None => Cow::from("UCI7OjJy-l1QAYZYuPaJYbag"), // Guangyou's youtube channel id
        },
    };
    let (channel_title, last_upload_time) =
        get_last_upload_time(youtube_api_key, &channel_id).await?;

    // TODO: somehow get a fucking localized duration string
    // let formatter = RelativeTimeFormatter::try_new_long_second(
    //     locale!("zh-Hant").into(),
    //     RelativeTimeFormatterOptions {
    //         numeric: Numeric::Auto,
    //     },
    // )?;
    let now = UtcDateTime::from_unix_timestamp((js_sys::Date::now() / 1000.0) as i64)?;
    worker::console_log!("now: {now}, last_upload_time: {last_upload_time}");
    let time_delta = now - last_upload_time;
    // let time_delta = formatter.format(Decimal::from((last_upload_time - now).whole_seconds()));
    Ok(format!(
        "**{channel_title}** 已經 **{time_delta}** 沒有發新影片了zz"
    ))
    // Ok(format!(
    //     "**{channel_title}** 上次發新影片已經是 **{time_delta}** 了zz"
    // ))
}

async fn get_last_upload_time(
    youtube_api_key: &str,
    channel_id: &str,
) -> Result<(String, UtcDateTime), Box<dyn Error>> {
    let mut response = youtube_api_fetch(
        "channels",
        &[("part", "contentDetails"), ("id", channel_id)],
        youtube_api_key,
    )?
    .send()
    .await?;
    let body = response.json::<serde_json::Value>().await?;
    let uploads_playlist_id = body
        .get("items")
        .and_then(|x| x.get(0))
        .and_then(|x| x.get("contentDetails"))
        .and_then(|x| x.get("relatedPlaylists"))
        .and_then(|x| x.get("uploads"))
        .and_then(|x| x.as_str())
        .ok_or_else(|| "cannot get the uploads playlist id from response body")?;

    let mut response = youtube_api_fetch(
        "playlistItems",
        &[
            ("part", "snippet"),
            ("playlistId", uploads_playlist_id),
            ("maxResults", "1"),
        ],
        youtube_api_key,
    )?
    .send()
    .await?;
    let body = response.json::<serde_json::Value>().await?;
    let channel_title = body
        .get("items")
        .and_then(|x| x.get(0))
        .and_then(|x| x.get("snippet"))
        .and_then(|x| x.get("channelTitle"))
        .and_then(|x| x.as_str())
        .ok_or_else(|| "cannot get channel title from response body")?;
    let last_published_at = body
        .get("items")
        .and_then(|x| x.get(0))
        .and_then(|x| x.get("snippet"))
        .and_then(|x| x.get("publishedAt"))
        .and_then(|x| x.as_str())
        .ok_or_else(|| "cannot get last publish time from response body")?;
    let last_published_at = UtcDateTime::parse(
        last_published_at,
        &time::format_description::well_known::Iso8601::DEFAULT,
    )?;

    Ok((channel_title.to_owned(), last_published_at))
}

async fn get_channel_id_from_keyword(
    youtube_api_key: &str,
    keyword: &str,
) -> Result<String, Box<dyn Error>> {
    let mut response = youtube_api_fetch(
        "search",
        &[
            ("part", "snippet"),
            ("type", "channel"),
            ("maxResults", "1"),
            ("q", keyword),
        ],
        youtube_api_key,
    )?
    .send()
    .await?;
    let body = response.json::<serde_json::Value>().await?;
    let channel_id = body
        .get("items")
        .and_then(|x| x.get(0))
        .and_then(|x| x.get("id"))
        .and_then(|x| x.get("channelId"))
        .and_then(|x| x.as_str())
        .map(|x| x.to_owned())
        .ok_or_else(|| "connot get channel id from response body")?;

    Ok(channel_id)
}

fn youtube_api_fetch(
    resource: &str,
    params: &[(&str, &str)],
    api_key: &str,
) -> Result<worker::Fetch, url::ParseError> {
    worker::Url::parse_with_params(
        &format!("https://www.googleapis.com/youtube/v3/{resource}"),
        params.into_iter().chain(std::iter::once(&("key", api_key))),
    )
    .map(worker::Fetch::Url)
}
