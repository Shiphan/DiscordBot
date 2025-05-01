FROM python:3.12

WORKDIR /usr/src/app

COPY bot.py .
COPY cogs/events_timer.py cogs/
COPY cogs/upload_timer.py cogs/

RUN pip install py-cord
RUN pip install requests

CMD [ "python", "bot.py" ]
