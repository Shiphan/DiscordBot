# DiscordBot

Rust + worker-rs + serenity

## Deploy

0. Dependencies: worker-build, wrangler
    ```
    nix develop -c $SHELL
    ```
1. Register commands:
    ```
    cargo run -p register
    ```
2. Set up secrets:
    ```
    wrangler secret put DISCORD_PUBLIC_KEY
    wrangler secret put YOUTUBE_API_KEY
    ```
3. Deploy to worker:
    ```
    wrangler deploy
    ```
4. Set bot's interactions endpoint URL:
    Settings -> General Information -> interactions endpoint URL

## Develop

Just deploy to production and test it

