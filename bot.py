import discord
import os
import requests
from datetime import datetime, timedelta
from dotenv import load_dotenv

load_dotenv()
bot = discord.Bot()


@bot.event
async def on_ready():
    print(f"{bot.user} is ready and online!")

class GetChannelInfo:
    apiKey = os.getenv('API_KEY')
    def __init__(self, channelId):
        try:
            data = self.GetJson(f"channels?part=snippet&id={channelId}")
            self.name = data["items"][0]["snippet"]["localized"]["title"]
        except:
            self.name = None
        try:
            data = self.GetJson(f"channels?part=contentDetails&id={channelId}")
            relatedPlaylistsId = data["items"][0]["contentDetails"]["relatedPlaylists"]["uploads"]
            try:
                data = self.GetJson(f"playlistItems?part=snippet&playlistId={relatedPlaylistsId}&maxResults=1")
                self.time = datetime.strptime(data["items"][0]["snippet"]["publishedAt"], "%Y-%m-%dT%H:%M:%SZ")
            except:
                self.time = None
        except:
            self.time = None

    def GetJson(self, path, apiKey = apiKey):
        try:
            data = requests.get(f"https://www.googleapis.com/youtube/v3/{path}&key={apiKey}").json()
            return data
        except:
            return None

@bot.slash_command(name = "posttimer", description = "Only supports Guangyou's channel at the moment.")
async def posttimer(ctx):
    channelId = "UCI7OjJy-l1QAYZYuPaJYbag"
    try:
        getChannelInfo = GetChannelInfo(channelId)
        channelName = getChannelInfo.name
        postTime = getChannelInfo.time
        timer = datetime.utcnow() - postTime
        timer -= timedelta(microseconds = timer.microseconds)
        #await ctx.respond(f"It's been {timer} since {channelName}'s last post.")
        await ctx.respond(f"**{channelName}** 已經 **{timer}** 沒有發新影片了zz")
    except:
        await ctx.respond("Channel or video not found.")
    print(f"{channelName}, {postTime}, {datetime.utcnow()},{timer}")

bot.run(os.getenv('TOKEN'))