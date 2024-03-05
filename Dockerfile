FROM python:3.11

WORKDIR /usr/src/app

COPY . .

RUN pip install py-cord
RUN pip install python-dotenv
RUN pip install requests

CMD [ "python", "./bot.py" ]