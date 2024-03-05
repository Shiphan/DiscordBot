import discord
import os
from dotenv import load_dotenv

load_dotenv()
bot = discord.Bot()

@bot.event
async def on_ready():
    print(f'{bot.user} is ready and online!')

# bot.load_extensions('cogs', recursive=True)
bot.load_extension('cogs.events_timer')
bot.load_extension('cogs.upload_timer')


bot.run(os.getenv('TOKEN'))