#!/usr/bin/env python3
import markdown as md
import os


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
    files: list[str] = collect_note_files("./papers")
    for file in files:
        with open(file, "r") as f:
            text: str = f.read()
            md_obj: md.Markdown = md.Markdown(extensions=["meta"])
            md_obj.convert(text)
            meta: dict[str, str] = md_obj.Meta

            entry = "\n  ".join(meta["bibtex"][:-1])
            entry += "\n" + meta["bibtex"][-1]
            print(f"{entry}\n")


if __name__ == "__main__":
    main()
