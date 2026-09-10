#!/bin/sh

surreal sql -u chat_app_owner -p "security" --pretty -e http://127.0.0.1:8000
