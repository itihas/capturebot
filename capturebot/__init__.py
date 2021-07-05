#!/usr/bin/env python
# pylint: disable=C0116
# based on echobot example in python-telegram-bot

import logging
import os
import urllib.parse
import urllib.request
import pkgutil
import sys, getopt
# import lxml.html as lh

from telegram import Update, MessageEntity
from telegram.ext import Updater, CommandHandler, MessageHandler, Filters, ConversationHandler, InlineQueryHandler, CallbackContext


TITLE, BODY = range(2)


# Enable logging
logging.basicConfig(
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s', level=logging.INFO
)

logger = logging.getLogger(__name__)


def cmd(params):
    c = "emacsclient -c -F \"((name . \\\"emacs-capture\\\") (height . 10) (width . 10))\" \"org-protocol://capture?{}\"".format(urllib.parse.urlencode(params, quote_via=urllib.parse.quote))
    logger.info(c)
    os.system(c)
    
# Define a few command handlers. These usually take the two arguments update and
# context. Error handlers also receive the raised TelegramError object in error.
def start(update: Update, _: CallbackContext) -> None:
    """Send a message when the command /start is issued."""
    update.message.reply_text('Hi!')


def help_command(update: Update, _: CallbackContext) -> None:
    """Send a message when the command /help is issued."""
    update.message.reply_text('Help!')


def echo(update: Update, _: CallbackContext) -> None:
    """Echo the user message."""
    update.message.reply_text(update.message.text)

def capture(update: Update, context: CallbackContext) -> int:
    update.message.reply_text("title?")
    return TITLE

def title(update: Update, context: CallbackContext) -> int:
    context.user_data['title'] = update.message.text
    update.message.reply_text("notes?")
    return BODY

def bookmark(update: Update, context: CallbackContext) -> int:
    data = update.message.parse_entities(types=[MessageEntity.URL]).values()
    for url in data:
        page = urllib.request.urlopen(url)
        # TODO archivebox or goose
        title = parse(page).find(".//title").text
        params = { 'template': 'L', 'url' : url, 'title': title, 'body': update.message.text }
        cmd(params)
    return ConversationHandler.END

    
def body(update: Update, context: CallbackContext) -> int:
    params = { 'template': 'C', 'title' : context.user_data['title'], 'body' : update.message.text }
    cmd(params)
    context.user_data.clear()
    return ConversationHandler.END

def cancel(u:Update, _:CallbackContext) -> int:
    return ConversationHandler.END

def todo_capture(update: Update, context: CallbackContext) -> None:
    data = update.message.text.split('\n', 1)
    params = { 'template': 'T', 'title' : data[0], 'body': ''.join(data[1:]) }
    cmd(params)
    context.user_data.clear()

def oneline_capture(update: Update, context: CallbackContext) -> None:
    data = update.message.text.split('\n', 1)
    params = { 'template': 'C', 'title' : data[0], 'body': ''.join(data[1:]) }
    cmd(params)
    context.user_data.clear()


def main() -> None:

    helptext = '''capturebot 
    -t --token <token>
    -f --tokenfile <tokenfile>
    -u --users <authorized users>
    '''
    token = ''
    users = ''
    try:
        opts, args = getopt.getopt(sys.argv[1:], "ht:f:u:",["help", "token=", "tokenfile=", "users="])
    except getopt.GetoptError:
        print(helptext)
    for opt, arg in opts:
        if opt in ("-h", "--help"):
            print(helptext)
        elif opt in ("-t", "--token"):
            token = arg
        elif opt in ("-f", "--tokenfile"):
            with open(arg) as f:
                token = f.read()
        elif opt in ("-u", "--users"):
            users = arg.split(',')

    """Start the bot."""
    # Create the Updater and pass it your bot's token.
    # token = pkgutil.get_data(__name__,"data/token").decode()
    updater = Updater(token, use_context=True)
    # with open("./data/token") as f:
    #     updater = Updater(f.read())

    # Get the dispatcher to register handlers
    dispatcher = updater.dispatcher

    # conversation
    # conv_handler = ConversationHandler(
    #     entry_points=[CommandHandler('big',capture)],
    #     states={
    #         TITLE:[MessageHandler(Filters.text & ~Filters.command, title)],
    #         BODY:[MessageHandler(Filters.text & ~Filters.command, body)],
    #         # TODO: tags, dates, todo status
    #     },
    #     fallbacks=[CommandHandler('cancel',cancel)],
    # )

    dispatcher.add_handler(conv_handler)

    dispatcher.add_handler(MessageHandler(Filters.usernames(users) & Filters.entity(MessageEntity.URL), bookmark))
    dispatcher.add_handler(MessageHandler(Filters.usernames(users) & Filters.regex('todo.*'), todo_capture))

    # # on different commands - answer in Telegram
    # dispatcher.add_handler(CommandHandler("start", start))
    # dispatcher.add_handler(CommandHandler("help", help_command))

    # on noncommand i.e message - echo the message on Telegram
    dispatcher.add_handler(MessageHandler(Filters.usernames(users) & Filters.text & ~Filters.command, oneline_capture))

    # Start the Bot
    updater.start_polling()

    # Run the bot until you press Ctrl-C or the process receives SIGINT,
    # SIGTERM or SIGABRT. This should be used most of the time, since
    # start_polling() is non-blocking and will stop the bot gracefully.
    updater.idle()


if __name__ == '__main__':
    main()
