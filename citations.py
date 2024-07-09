#!/usr/bin/env python3
import os
import sys
import re
import yaml
import argparse

from config import get_config

header_re = re.compile(r"---\n([\s\S]*)\n---\n", flags=re.MULTILINE)
first_line_re = re.compile(r"@(.+?){(.+?), ")

def collect_note_files(dir: str) -> list[str]:
    files: list[str] = []
    for elem in os.scandir(dir):
        if elem.is_dir():
            files.extend(collect_note_files(elem.path))
            continue
        elif elem.is_file():
            if elem.name.endswith(".md"):
                files.append(elem.path)
        else:
            print(f"[WARN] unexpected node type in folder: {elem}")
    return files


def get_args():
    parser = argparse.ArgumentParser(
        prog="envy.citations",
        description="""Serve a collection of markdown files in the browser""",
    )
    parser.add_argument(
        "-u", "--use-config", help="Path to configuration file", type=str, default=None
    )
    parser.add_argument(
        "-c", "--config-help", help="Show config file help", action="store_true"
    )
    return parser.parse_args()



def main():
    args = get_args()
    cfg = get_config(args.use_config)

    files: list[str] = collect_note_files(cfg.root_dir)
    for file in files:
        with open(file, "r") as f:
            text: str = f.read()

            match = header_re.match(text)
            if match is not None:
                match = match.group(1)
                try:
                    header = yaml.load(match, Loader=yaml.CLoader)
                except Exception as e:
                    print(f"Could not parse yaml header for {file}: {e}", file=sys.stderr)
                    continue

                meta: dict[str, str] = header

                entry = first_line_re.sub(r"@\1{\2,\n  ", meta["bibtex"])
                entry = entry.replace("}, ", "},\n  ")
                entry = entry.replace("} ", "}\n")
                print(f"{entry}\n")


if __name__ == "__main__":
    main()
