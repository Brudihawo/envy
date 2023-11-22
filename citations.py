#!/usr/bin/env python3
import os
import re
import yaml

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


def main():
    files: list[str] = collect_note_files("./notes/papers")
    for file in files:
        with open(file, "r") as f:
            text: str = f.read()

            match = header_re.match(text)
            if match is not None:
                match = match.group(1)
                header = yaml.load(match, Loader=yaml.CLoader)
                meta: dict[str, str] = header

                entry = first_line_re.sub(r"@\1{\2,\n  ", meta["bibtex"])
                entry = entry.replace("}, ", "},\n  ")
                entry = entry.replace("} ", "}\n")
                print(f"{entry}\n")


if __name__ == "__main__":
    main()
