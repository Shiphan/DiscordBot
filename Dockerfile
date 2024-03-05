FROM python:3.11

WORKDIR /src

COPY . .

RUN pip install py-cord
RUN pip install python-dotenv
RUN pip install requests

CMD [ "python", "./bot.py" ]