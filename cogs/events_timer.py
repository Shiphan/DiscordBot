import discord
from discord.ext import commands
from discord import option  

import json
from datetime import date, datetime, timedelta
from typing import Union
import re

from discord.ui.item import Item


class Events:
    def __init__(self, eventsJsonPath = 'data/events.json'):
        self.eventsJsonPath = eventsJsonPath
        try:
            with open(self.eventsJsonPath, encoding= 'utf-8') as f:
                self.list = json.load(f)
        except:
            self.list = []
        print(f'events: {len(self.list)} event(s) loaded.')

    def addEvent(self, userid: int, name: str, topic: str, starttime: Union[datetime, str]):
        if isinstance(starttime, datetime):
            starttime = starttime.strftime('%Y/%m/%d-%H:%M')
        self.list.append({'userid': userid, 'name': name, 'topic': topic, 'starttime': starttime})
        with open(self.eventsJsonPath, 'w+', encoding= 'utf-8') as f:
            f.write(json.dumps(self.list, sort_keys=True, indent=4))

    def removeEvent(self, index: int): 
        try:
            self.list.pop(index)
            with open(self.eventsJsonPath, 'w+', encoding= 'utf-8') as f:
                f.write(json.dumps(self.list, sort_keys=True, indent=4))
        except:
            print('removeEvent: Index out of range.')

def deltafstr(time: str):
    if re.fullmatch(r'^\d{4}/\d{2}/\d{2}$', time):
        timer = datetime(year=date.today().year, month=date.today().month, day=date.today().day) - datetime.strptime(time, '%Y/%m/%d')
    elif re.fullmatch(r'^\d{4}/\d{2}/\d{2}-\d{2}:\d{2}$', time):
        timer = datetime.now() - datetime.strptime(time, '%Y/%m/%d-%H:%M')
    return timer

def respondfevent(event: dict, name: str):
    try:
        starttime = event["starttime"]
        timer = deltafstr(starttime)
        if re.fullmatch(r'^\d{4}/\d{2}/\d{2}$', starttime):
            if timer == timedelta():
                return f'{name}今天會{event["topic"]}'
            elif timer > timedelta():
                timer = timer.days
                return f'{name}已經{event["topic"]}{timer}天了'
            else:
                timer = timer.days
                return f'{name}在{-timer}天後開始{event["topic"]}'
        else:
            if timer >= timedelta():
                timer = str(timer).split('.', 2)[0]
                return f'{name}已經{event["topic"]}{timer}了'
            else:
                timer = str(-timer).split('.', 2)[0]
                return f'{name}在{timer}後開始{event["topic"]}'
    except:
        return f'{name}從{event["starttime"]}開始{event["topic"]}'

events = Events()

async def updateEmbed(interaction: discord.Interaction, deferred: bool = False, ephemeral: bool = True, listIsEphemeral: bool = True):
    if not deferred:
        await interaction.response.defer()    
    if ephemeral:
        if len(events.list) == 0:
            await interaction.edit_original_response(content='現在沒有任何活動', embed=None, view=TimerListView(isNoEvent=True, isEphemeral=listIsEphemeral))
        else:
            embed = discord.Embed(title='All events', timestamp=datetime.today())
            for idx, event in enumerate(events.list, start=1):
                embed.add_field(name=f'#{idx}', value=f'{event["name"]}從{event["starttime"]}開始{event["topic"]}', inline=False)
            await interaction.edit_original_response(content=None, embed=embed, view=TimerListView(isEphemeral=listIsEphemeral))
        print(f'events-list: list is updated, {len(events.list)} event(s) listed.')
    else:
        await interaction.delete_original_response()
        if len(events.list) == 0:
            await interaction.followup.send('現在沒有任何活動', embed=None, view=TimerListView(isNoEvent=True, isEphemeral=False))
        else:
            embed = discord.Embed(title='All events', timestamp=datetime.today())
            for idx, event in enumerate(events.list, start=1):
                embed.add_field(name=f'#{idx}', value=f'{event["name"]}從{event["starttime"]}開始{event["topic"]}', inline=False)
            await interaction.followup.send(content=None, embed=embed, view=TimerListView(isEphemeral=False))
        print(f'events-list: list is updated and public, {len(events.list)} event(s) listed.')

# events list
class ShowButton(discord.ui.Button):
    def __init__(self, *, style: discord.ButtonStyle = discord.ButtonStyle.secondary, label: str | None = None, disabled: bool = False, custom_id: str | None = None, url: str | None = None, emoji: str | discord.Emoji | discord.PartialEmoji | None = None, row: int | None = None, listIsEphemeral: bool):
        super().__init__(style=style, label=label, disabled=disabled, custom_id=custom_id, url=url, emoji=emoji, row=row)
        self.listIsEphemeral = listIsEphemeral

    async def callback(self, interaction: discord.Interaction):
        await interaction.response.defer()
        await interaction.followup.send(view=TimerView(), ephemeral=True)
        print('show-timer-button: showed events select menu.')
        await updateEmbed(interaction=interaction, listIsEphemeral=self.listIsEphemeral, deferred=True)
        return await super().callback(interaction)
    
class RemoveButton(discord.ui.Button):
    def __init__(self, *, style: discord.ButtonStyle = discord.ButtonStyle.secondary, label: str | None = None, disabled: bool = False, custom_id: str | None = None, url: str | None = None, emoji: str | discord.Emoji | discord.PartialEmoji | None = None, row: int | None = None, listIsEphemeral: bool):
        super().__init__(style=style, label=label, disabled=disabled, custom_id=custom_id, url=url, emoji=emoji, row=row)
        self.listIsEphemeral = listIsEphemeral

    async def callback(self, interaction: discord.Interaction):
        await interaction.response.defer()
        await interaction.followup.send(view=RemoveView(viewInteraction=interaction, listIsEphemeral=self.listIsEphemeral), ephemeral=True)
        print('remove-event-button: showed events select menu.')
        return await super().callback(interaction)

class UpdateButton(discord.ui.Button):
    def __init__(self, *, style: discord.ButtonStyle = discord.ButtonStyle.secondary, label: str | None = None, disabled: bool = False, custom_id: str | None = None, url: str | None = None, emoji: str | discord.Emoji | discord.PartialEmoji | None = None, row: int | None = None, listIsEphemeral: bool):
        super().__init__(style=style, label=label, disabled=disabled, custom_id=custom_id, url=url, emoji=emoji, row=row)
        self.listIsEphemeral = listIsEphemeral

    async def callback(self, interaction: discord.Interaction):
        await updateEmbed(interaction=interaction, listIsEphemeral=self.listIsEphemeral)
        return await super().callback(interaction)
    
class PublicButton(discord.ui.Button):
    def __init__(self, *, style: discord.ButtonStyle = discord.ButtonStyle.secondary, label: str | None = None, disabled: bool = False, custom_id: str | None = None, url: str | None = None, emoji: str | discord.Emoji | discord.PartialEmoji | None = None, row: int | None = None):
        super().__init__(style=style, label=label, disabled=disabled, custom_id=custom_id, url=url, emoji=emoji, row=row)

    async def callback(self, interaction: discord.Interaction):
        await updateEmbed(interaction=interaction, ephemeral=False)
        return await super().callback(interaction)

class TimerListView(discord.ui.View):
    def __init__(self, *items: Item, timeout: float | None = 180, disable_on_timeout: bool = True, isNoEvent: bool = False, isEphemeral: bool = True):
        super().__init__(*items, timeout=timeout, disable_on_timeout=disable_on_timeout)
        self.add_item(ShowButton(label='顯示時間', style=discord.ButtonStyle.primary, emoji='⏱', disabled=isNoEvent, listIsEphemeral=isEphemeral))
        self.add_item(RemoveButton(label='刪除', style=discord.ButtonStyle.danger, emoji='🗑', disabled=isNoEvent, listIsEphemeral=isEphemeral))
        self.add_item(UpdateButton(label='更新', style=discord.ButtonStyle.secondary, emoji='🔄', listIsEphemeral=isEphemeral))
        self.add_item(PublicButton(label='公開', style=discord.ButtonStyle.secondary, disabled=not isEphemeral))

# event timer
class TimerSelect(discord.ui.Select):
    def __init__(self, select_type: discord.ComponentType = discord.ComponentType.string_select, *, custom_id: str | None = None, placeholder: str | None = None, min_values: int = 1, max_values: int = 1, options: list[discord.SelectOption] = None, channel_types: list[discord.ChannelType] = None, disabled: bool = False, row: int | None = None) -> None:
        super().__init__(select_type, custom_id=custom_id, placeholder=placeholder, min_values=min_values, max_values=max_values, options=options, channel_types=channel_types, disabled=disabled, row=row)

    async def callback(self, interaction: discord.Interaction):
        idx = int(self.values[0])
        await interaction.response.defer()
        await interaction.delete_original_response()
        try:
            event = events.list[idx]
            await interaction.followup.send(respondfevent(event=event, name=f'<@{event["userid"]}>'))
            print(f'event-timer: {event["name"]}, {event["userid"]}, {event["topic"]}, {event["starttime"]}, {deltafstr(event["starttime"])}')
        except:
            await interaction.followup.send('No matching event found.', ephemeral=True)
            print('event-timer: No matching event found.')
        return await super().callback(interaction)

class TimerView(discord.ui.View):
    def __init__(self, *items: Item, timeout: float | None = 180, disable_on_timeout: bool = True):
        super().__init__(*items, timeout=timeout, disable_on_timeout=disable_on_timeout)
        self.add_item(TimerSelect(options=[discord.SelectOption(label=f'{d["name"]}從{d["starttime"]}開始{d["topic"]}', description=f'#{idx+1}', value=str(idx)) for idx, d in enumerate(events.list)]))

# remove event
class RemoveSelect(discord.ui.Select):
    def __init__(self, select_type: discord.ComponentType = discord.ComponentType.string_select, *, custom_id: str | None = None, placeholder: str | None = None, min_values: int = 1, max_values: int = 1, options: list[discord.SelectOption] = None, channel_types: list[discord.ChannelType] = None, disabled: bool = False, row: int | None = None, viewInteraction: discord.Interaction, listIsEphemeral: bool) -> None:
        super().__init__(select_type, custom_id=custom_id, placeholder=placeholder, min_values=min_values, max_values=max_values, options=options, channel_types=channel_types, disabled=disabled, row=row)
        self.viewInteraction = viewInteraction
        self.listIsEphemeral = listIsEphemeral

    async def callback(self, interaction: discord.Interaction):
        idx = int(self.values[0])
        await interaction.response.defer()
        await interaction.delete_original_response()
        try:
            event = events.list[idx]
            events.removeEvent(idx)
            await interaction.followup.send(f'Removed an event: <@{event["userid"]}>從{event["starttime"]}開始{event["topic"]}')
            print(f'remove-event-select: removed {event["name"]}, {event["userid"]}, {event["topic"]}, {event["starttime"]}')
        except:
            await interaction.followup.send('No matching event found.', ephemeral=True)
            print('remove-event-select: No matching event found.')
        await updateEmbed(interaction=self.viewInteraction, deferred=True, listIsEphemeral=self.listIsEphemeral)
        return await super().callback(interaction)

class RemoveView(discord.ui.View):
    def __init__(self, *items: Item, timeout: float | None = 180, disable_on_timeout: bool = True, viewInteraction: discord.Interaction, listIsEphemeral: bool):
        super().__init__(*items, timeout=timeout, disable_on_timeout=disable_on_timeout)
        self.add_item(RemoveSelect(viewInteraction=viewInteraction, listIsEphemeral=listIsEphemeral, placeholder='選擇你要移除的事件', options=[discord.SelectOption(label=f'{d["name"]}從{d["starttime"]}開始{d["topic"]}', description=f'#{idx+1}', value=str(idx)) for idx, d in enumerate(events.list)]))

# all slash commands
class EventsTimer(commands.Cog):
    def __init__(self, bot):
        self.bot = bot

    @commands.slash_command(name='event-timer', description='')
    async def eventTimer(self, ctx):
        await ctx.respond(view=TimerView(), ephemeral=True)
        print('event-timer: showed events select menu.')

    @commands.slash_command(name='events-list', description='取得所有事件的列表')
    async def eventsList(self, ctx):
        if len(events.list) == 0:
            await ctx.respond('現在沒有任何活動', view=TimerListView(isNoEvent=True), ephemeral=True)
        else:
            embed = discord.Embed(title='All events', timestamp=datetime.today())
            for idx, event in enumerate(events.list, start=1):
                embed.add_field(name=f'#{idx}', value=f'{event["name"]}從{event["starttime"]}開始{event["topic"]}', inline=False)
            await ctx.respond(embed=embed, view=TimerListView(), ephemeral=True)
        print(f'events-list: {len(events.list)} event(s) listed.')

    @commands.slash_command(name='search-events', description='搜尋')
    @option('user', default=None)
    @option('topic', default=None)
    @option('starttime', description='"yyyy/mm/dd" or "yyyy/mm/dd-hh:mm"', default=None)
    async def searchEvents(self, ctx, user: discord.Member, topic: str, starttime: str):
        if not user and not topic and not starttime:
            await ctx.respond('在選項中輸入關鍵字以搜尋', ephemeral=True)
            print('timer-search: no keywords')
        else:
            if user:
                filterUser = [user.id == l["userid"] for l in events.list]
            else:
                filterUser = [True] * len(events.list)
            if topic:
                filterTopic = [bool(re.search(topic, l["topic"])) for l in events.list]
            else:
                filterTopic = [True] * len(events.list)
            if starttime:
                try:
                    deltafstr(starttime)
                    filterStartTime = [bool(re.match(starttime, l["starttime"])) for l in events.list]
                except:
                    filterStartTime = [True] * len(events.list)
            else:
                filterStartTime = [True] * len(events.list)
            resultList = [s for b, s in zip([a and b and c for a, b , c in zip(filterUser, filterTopic, filterStartTime)], events.list) if b]
            if len(resultList) == 0:
                await ctx.respond('No matching event found.', ephemeral=True)
                print('timer-search: No matching event found.')
            elif len(resultList) == 1:
                event = resultList[0]
                await ctx.respond(respondfevent(event=event, name=f'<@{event["userid"]}>'))
                print(f'timer-search: {event["name"]}, {event["userid"]}, {event["topic"]}, {event["starttime"]}, {deltafstr(event["starttime"])}')
            else:
                respond = '、'.join([respondfevent(event=event, name=f'<@{event["userid"]}>') for event in resultList])
                await ctx.respond(respond)
                printResult = ', '.join([f'[{event["name"]}, {event["userid"]}, {event["topic"]}, {event["starttime"]}, {deltafstr(event["starttime"])}]' for event in resultList])
                print(f'timer-search: {printResult}')

    @commands.slash_command(name='add-event', description='新增')
    @option('user')
    @option('topic')
    @option('starttime', description='"yyyy/mm/dd" or "yyyy/mm/dd-hh:mm" or today or now')
    async def addEvent(self, ctx, user: discord.Member, topic: str, starttime: str):
        if starttime == 'today':
            starttime = datetime.today().strftime('%Y/%m/%d')
        elif starttime == 'now':
            starttime = datetime.today().strftime('%Y/%m/%d-%H:%M')
        if re.fullmatch(r'^\d{4}/\d{2}/\d{2}$', starttime) or re.fullmatch(r'^\d{4}/\d{2}/\d{2}-\d{2}:\d{2}$', starttime):
            try:
                deltafstr(starttime)
                name = user.nick or user.display_name or user.name
                events.addEvent(userid=user.id, name=name, topic=topic, starttime=starttime)
                await ctx.respond(f'Added an event: <@{user.id}>從{starttime}開始{topic}')
                print(f'add-event: {name}, {user.id}, {topic}, {starttime}')
            except:
                await ctx.respond('這是三小時間', ephemeral=True)
                print('add-event: starttime is not an existing time')
        else:
            await ctx.respond('starttime didn\'t match the pattern.', ephemeral=True)
            print('add-event: starttime didn\'t match the pattern.')

def setup(bot):
    bot.add_cog(EventsTimer(bot))