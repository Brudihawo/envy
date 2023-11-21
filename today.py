#!/usr/bin/env python3

import os
import sys
import subprocess as sp
from datetime import date, datetime, timedelta
from sys import argv
from calendar import TextCalendar

START_DATE: date = datetime.strptime("01.11.23", "%d.%m.%y").date()
NEXT_EMPTY: str = "[next](<empty>)"
DIRPATH: str = "notes/daily"


def open_in_editor(fname: str):
    editor: str = os.getenv("EDITOR") or "nvim"
    cmd = [editor, f"{fname}"]
    print("executing " + " ".join(cmd))
    sp.call(cmd)


def cal_today() -> str:
    now = datetime.now()
    cal: str = TextCalendar().formatmonth(now.year, now.month, w=4)
    cal = "\n ".join(cal.split("\n"))

    day = now.day
    if len(str(day)) == 2:
        cal = cal.replace(f" {day} ", f"[{day}]")
    else:
        cal = cal.replace(f"   {day} ", f"[  {day}]")

    return cal


def update_file_with_next(fname: str, next_fname: str):
    with open(fname, "r") as f:
        contents = f.read()

    contents = contents.replace(NEXT_EMPTY, f"[next]({next_fname})")
    with open(fname, "w") as f:
        f.write(contents)


def today_filename() -> str:
    now: datetime = datetime.now()
    return os.path.join(DIRPATH, f"{now:%y-%m-%d}.md")


def open_today():
    now: datetime = datetime.now()
    fname: str = today_filename()
    if not os.path.exists(fname):
        with open(fname, "w") as f:
            f.write(r"```" + "\n")
            f.write(cal_today())
            f.write(r"```" + "\n")
            f.write(f"# {now:%d.%m.%y}\n\n")
            le = last_entry()
            if le is not None:
                f.write(f"[last]({le})\n")
                update_file_with_next(le, fname)
            else:
                print("Did not find last entry", file=sys.stderr)
            f.write(NEXT_EMPTY)
    open_in_editor(fname)


def last_entry() -> str | None:
    cur_date: date = datetime.now().date()
    while cur_date > START_DATE:
        cur_date: date = cur_date - timedelta(days=1.0)
        fname = f"{DIRPATH}/{cur_date:%y-%m-%d}.md"
        if os.path.exists(fname):
            return fname
    return None


def open_last():
    le = last_entry()
    if le is not None:
        open_in_editor(le)
        return

    print("no valid entries")
    exit(1)


def print_help():
    program: str = argv[0]
    usage = f"""{program} - daily notes
usage:
    {program}             - open or create and open new today note file
    {program} --last      - open last created note before today"""
    print(usage)


def main():
    if len(argv) == 1:
        open_today()
    elif len(argv) == 2 and argv[1] == "--last":
        open_last()
    else:
        print_help()


if __name__ == "__main__":
    main()