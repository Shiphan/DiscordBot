import discord
from discord.ext import commands
from discord import option

import os
import requests
from datetime import datetime, timedelta


class ChannelInfo:
    apiKey = os.getenv('API_KEY')

    def __init__(self, name: str, lastUploadTime: datetime):
        self.name = name
        self.lastUploadTime = lastUploadTime

    @classmethod
    def getFromId(cls, channelId):
        try:
            data = cls.getJson(path=f'channels?part=contentDetails&id={channelId}')
            relatedPlaylistsId = data["items"][0]["contentDetails"]["relatedPlaylists"]["uploads"]
            data = cls.getJson(path=f'playlistItems?part=snippet&playlistId={relatedPlaylistsId}&maxResults=1')
            name = data["items"][0]["snippet"]["channelTitle"]
            lastUploadTime = datetime.strptime(data["items"][0]["snippet"]["publishedAt"], "%Y-%m-%dT%H:%M:%SZ")
        except:
            name = None
            lastUploadTime = None
        return cls(name=name, lastUploadTime=lastUploadTime)
    
    @classmethod
    def getFromSearch(cls, keyword):
        try:
            data = cls.getJson(path=f'search?part=snippet&type=channel&maxResults=1&q={keyword}')
            channelId = data["items"][0]["id"]["channelId"]
            return cls.getFromId(channelId=channelId)
        except:
            return cls(name=None, lastUploadTime=None)

    def getJson(path, apiKey = apiKey):
        try:
            return requests.get(f"https://www.googleapis.com/youtube/v3/{path}&key={apiKey}").json()
        except:
            return None

class UploadTimer(commands.Cog):
    def __init__(self, bot):
        self.bot = bot

    @commands.slash_command(name='uploadtimer', description='How long it has been since the last upload. (Use Guangyou\'s YouTube as default.)')
    @option('channelid', description='YouTube channel ID', default=None)
    @option('search', description='The keyword to search for on Youtube', default=None)
    async def uploadtimer(self, ctx, channelid: str, search: str):
        if channelid:
            channelInfo = ChannelInfo.getFromId(channelId=channelid)
            if not channelInfo.name and search:
                channelInfo = ChannelInfo.getFromSearch(keyword=search)
        elif search:
            channelInfo = ChannelInfo.getFromSearch(keyword=search)
        else:
            channelInfo = ChannelInfo.getFromId(channelId='UCI7OjJy-l1QAYZYuPaJYbag')
        try:
            channelName = channelInfo.name
            postTime = channelInfo.lastUploadTime
            timer = datetime.utcnow() - postTime
            timer = str(timer).split('.', 2)[0]
            # await ctx.respond(f'It's been {timer} since {channelName}'s last post.')
            await ctx.respond(f'**{channelName}** 已經 **{timer}** 沒有發新影片了zz')
            print(f'uploadtimer: {channelName}, {postTime}, {datetime.utcnow()}, {timer}')
        except:
            if not channelInfo.name:
                await ctx.respond('Channel not found.', ephemeral=True)
                print('uploadtimer: Channel not found.')
            elif not channelInfo.lastUploadTime:
                await ctx.respond(f'**{channelInfo.name}**根本沒上傳影片...')
                print(f'uploadtimer: {channelInfo.name}, Video not found.')
            else:
                await ctx.respond('Channel or video not found.', ephemeral=True)
                print('uploadtimer: Channel or video not found.')


def setup(bot):
    bot.add_cog(UploadTimer(bot)) 
