import subprocess
import sys
import string
import platform
from base64 import b64decode
import urllib.request
import json
from functools import cache
from datetime import datetime
from collections import namedtuple
from typing import Optional, Iterator, List
import re

ParsedToken = namedtuple("ParsedToken", ["raw", "userid", "created", "hmac"])
DB_FILTER = ["chrome", "vivaldi", "discord"]
_urlsafe_decode_translation = str.maketrans("-_", "+/")


def round_down(num, divisor):
    return num - (num % divisor)

def urlsafe_b64decode(s: str):
    s = s.translate(_urlsafe_decode_translation)
    return b64decode(s, validate=True)


@cache
def id2username(id: str) -> str:
    try:
        resp = urllib.request.urlopen(
            "https://terminal-discord.vercel.app/api/lookup-user?id={}".format(id)
        )
        data = json.load(resp)
        return data.get("username") or "Unknown"
    except:
        return "Unknown"


def parseIdPart(id_part: str) -> str:
    return urlsafe_b64decode(id_part).decode()


def parseTimePart(time_part: str) -> datetime:
    if len(time_part) < 6:
        raise Exception("Time part too short")
    padded_time_part = time_part + "=" * (
        (round_down(len(time_part), 4) + 4) - len(time_part)
    )
    decoded = urlsafe_b64decode(padded_time_part)
    timestamp = sum((item * 256 ** idx for idx, item in enumerate(reversed(decoded))))
    if timestamp < 1293840000:
        timestamp += 1293840000
    return datetime.fromtimestamp(timestamp)


def parseToken(token: str) -> ParsedToken:
    parts = token.split(".")
    return ParsedToken(
        raw=token,
        userid=parseIdPart(parts[0]),
        created=parseTimePart(parts[1]),
        hmac=parts[2],
    )


def run_command(cmd: str) -> List[str]:
    output = subprocess.Popen(
        [cmd], shell=True, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL
    )
    return output.communicate()[0].decode().splitlines()


def main():
    skip_username_lookup = "--no-lookup" in sys.argv
    print("Searching for Discord localstorage databases...")
    # First, we search for .ldb files, these are the leveldb files used by chromium to store localstorage data,
    # which contains the discord token.
    rg = False
    # Try and use ripgrep, because it's much faster, otherwise, fallback to `find`.
    try:
        subprocess.check_output(["rg", "--version"])
        results = run_command("rg ~/ --files -g '*.ldb'")
        rg = True
    except FileNotFoundError:
        results = run_command("find ~/ -name '*.ldb'")

    if len(results) == 0 and rg:
        # Try again, but search hidden directories.
        results = run_command("rg ~/ --hidden --files -g '*.ldb'")

    if len(results) == 0:
        print("No databases found.")
        sys.exit(1)

    # Only search for tokens in local starage directories belonging known Chromium browsers or discord
    discord_databases = list(
        filter(
            lambda x: any([db in x.lower() for db in DB_FILTER])
            and "Local Storage" in x,
            results,
        )
    )

    # Then collect strings that look like discord tokens.
    token_candidates = set()
    token_re = re.compile(rb'([a-z0-9_-]{23,28}\.[a-z0-9_-]{6,7}\.[a-z0-9_-]{27})', flags=re.IGNORECASE)
    for database in discord_databases:
        for line in open(database, 'rb'):
            for result in token_re.finditer(line):
                try:
                    token_candidates.add(parseToken(result.group(0).decode()))
                except:
                    continue

    if len(token_candidates) == 0:
        print("No Discord tokens found")
        return

    print("Possible Discord tokens found (sorted newest to oldest):\n")
    token_candidates = sorted(token_candidates, key=lambda t: t.created, reverse=True)
    for token in token_candidates:
        if skip_username_lookup:
            print("{} created: {}".format(token.raw, token.created))
        else:
            print(
                "@{}: {} created: {}".format(
                    id2username(token.userid), token.raw, token.created
                )
            )


if __name__ == "__main__":
    main()
