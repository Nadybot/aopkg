# Packaging for aopkg

aopkg requires two files to be present in the ZIP files you upload: a `README.md` file and an `aopkg.toml`.

## The README.md

The `README.md` file should contain any valid markdown and will be rendered as the long description.

## The aopkg.toml

The `aopkg.toml` file should look like this:

```toml
name = "Package-Name"              # Alphanumeric, - or _ allowed. Maximum 30 characters.
description = "Does stuff"         # Any string. Maximum 100 characters.
version = "0.1.0"                  # Any semantic version number.
author = "Nadyita"                 # Any string. Maximum 30 characters.
bot_type = "Nadybot"               # Either "Nadybot", "Tyrbot", "Budabot" or "BeBot".
bot_version = "^5.0.0"             # Semantic version requirement of the bot.
```
