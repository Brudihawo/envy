#!/usr/bin/env python3

import os
import subprocess as sp
from bibtex_parser import Error, Parser

def open_in_editor(fname: str):
    editor: str = os.getenv("EDITOR") or "nvim"
    cmd = [editor, f"{fname}"]
    print("executing " + " ".join(cmd))
    sp.call(cmd)

def get_clipboard() -> str:
    p = sp.Popen(["xclip", "-selection", "clipboard", "-o"], stdout=sp.PIPE)
    r = p.wait()

    if r != 0 or p.stdout is None:
        print("Error getting clipboard")
        exit()

    data = p.stdout.read().decode()
    return data


if __name__ == "__main__":
    clipboard = get_clipboard().strip()
    parsed = Parser(clipboard).parse()
    if isinstance(parsed, Error):
        print(f"Error: Could not parse bibtex entry in clipboard: {a}. Clipboard contents:'{clipboard}'")
        exit(1)
    key = parsed.name
    note_path = os.path.expanduser(f"~/notes/papers/{key}.md")
    if os.path.exists(note_path):
        print(f"Error: Could not create file: Note '{note_path}' already exists")
        exit(1)

    reduced_entry = "\n        ".join(line.strip() for line in clipboard.split("\n"))
    doctitle = (parsed.get_or_none('title') or '<title>').strip('{}')
    with open(note_path, "w") as file:
        file.write(f"""---
bibtex: "{reduced_entry}"
pdf: "./doc/{key}.pdf"
tags: [unread]
---
# {doctitle}"""
        )
    open_in_editor(note_path)
