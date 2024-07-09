#!/usr/bin/env python
from functools import lru_cache
from http.server import HTTPServer, SimpleHTTPRequestHandler
import os
import shutil
import pycmarkgfm
import re
import yaml
import logging
import pyinotify
import subprocess
import datetime
import argparse
import json
import urllib.request
from functools import lru_cache
import shlex
import sys

from bibtex_parser import Parser, Entry
from config import get_config, Config, print_config_help

logger = logging.Logger("logger")
hn = logging.StreamHandler()
hn.setFormatter(logging.Formatter("%(asctime)s %(levelname)s %(message)s"))
logger.addHandler(hn)

link_re = re.compile(r"\((.*?)\.md(#.*){0,1}\)")
pdf_re = re.compile(r"\((.*?).pdf(#.*){0,1}\)")
empty_re = re.compile(r"\[next\]\(<empty>\)")
header_re = re.compile(r"---\n([\s\S]*)\n---\n", flags=re.MULTILINE)
script_path = os.path.dirname(__file__)

APP_NAME = "Envy"
ADDRESS = "localhost"
PORT = 6969


html_start = """<!DOCTYPE html>
<html>
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <meta charset="utf-8">
    <link rel="icon" type="image/x-icon" href="/assets/favicon.ico">
    <script>
        MathJax = {
            tex: {
                inlineMath: [['$', '$'], ['\\\\(', '\\\\)']]
            },
            svg: {
                fontCache: 'global'
            }
        };
    </script>
    <script async src="/assets/mathjax/es5/tex-chtml.js" id="MathJax-script"></script>"""
html_end = "</body>\n</html>"
markdown_insert = """<style>
.markdown-body {
    box-sizing: border-box;
    min-width: 200px;
    max-width: 980px;
    margin: 0 auto;
    padding: 45px;
}
@media (max-width: 767px) {
    .markdown-body {
        padding: 15px;
    }
}
</style>
"""

with open(os.path.join(script_path, "filter.js"), "r") as file:
    filter_script = file.read()
filter_script = f"""<script>\n{filter_script}\n</script>\n"""

logo_d = 32

assets_dir_name = "assets"
favicon_file_name = "favicon.ico"
css_file_name = "github-markdown-dark.css"
mathjax_assets_path = "./assets/mathjax/es5"



def copy_locals(cfg: Config):
    pass

def download_assets(cfg: Config):
    assets_path = os.path.join(cfg.serve_path, assets_dir_name)

    mathjax_path = os.path.join(assets_path, "mathjax")
    subprocess.call(
        f"git clone https://github.com/mathjax/MathJax.git {mathjax_path} --depth=1".split()
    )

    markdown_css_url = "https://raw.githubusercontent.com/sindresorhus/github-markdown-css/main/github-markdown-dark.css"
    with urllib.request.urlopen(markdown_css_url) as f:
        markdown_css = f.read().decode("utf-8")

    css_file_path = os.path.join(assets_path, css_file_name)

    if not os.path.exists(assets_path):
        os.makedirs(assets_path)
    with open(css_file_path, "w") as file:
        file.write(markdown_css)


def strip_value_or_empty(val: str | None) -> str:
    if val is not None:
        return val.strip("{}")

    return ""


def collect_structure(folder_path) -> tuple[list[str], list[str], list[str]]:
    files = []
    folders = []
    copy_docs = []
    for ent in os.scandir(folder_path):
        if ent.is_file():
            if ent.path.endswith(".md"):
                files.append(ent.path)
            elif ent.path.endswith(".pdf"):
                copy_docs.append(ent.path)
            elif ent.path.endswith(".png"):
                copy_docs.append(ent.path)
        elif ent.is_dir():
            folders.append(ent.path)
            sub_files, sub_docs, sub_folders = collect_structure(ent.path)
            files.extend(sub_files)
            folders.extend(sub_folders)
            copy_docs.extend(sub_docs)
        else:
            logger.debug(f"Ignoring {ent.path}")

    return (
        files,
        copy_docs,
        folders,
    )

def get_file_contents(path: str) -> str | None:
    with open(path, "r") as f:
        try:
            content = f.read()
        except UnicodeDecodeError as e:
            logger.error(f"Could not decode file: {path}: {e}")
            return None

    return content


def get_paper_meta(in_path):
    content = get_file_contents(in_path)
    if content is None:
        return None

    match = header_re.match(content)
    if match is None:
        return None

    try:
        header = match.group(1)# .replace("\\", "\\\\")
        header = yaml.load(header, Loader=yaml.CLoader)
    except Exception as e:
        logger.error(f"Invalid yaml in '{in_path}': {e}")
        return None

    return header


def generate_index(serve_path: str, css_file_path: str, files: list[str], root_dir):
    col_gap = 20
    cols = 5
    with open(os.path.join(serve_path, "index.html"), "w") as file:
        file.write(
            f"""{html_start}
<link rel="stylesheet" href="/{css_file_path}">
{markdown_insert}
<style>
.mcol_ul {{
  counter-reset: section;
  -moz-column-count: {cols};
  -moz-column-gap: {col_gap}px;
  -webkit-column-count: {cols};
  -webkit-column-count: {col_gap}px;
  column-count: {cols};
  column-gap: {col_gap}px;
}}

.mcol_li {{
  padding-left: 0px;
  position: relative;
}}

.mcol_li:before {{
  couner-increment: section;
  counter: counter(section) ".";
  margin: 0 0 0 -34px;
  text-align: right;
  width: 2em;
  display: inline-block;
  position: absolute;
  height: 100%;
}}
</style>
<title>{APP_NAME}: Collection of my Notes</title>
</head>
<body class="markdown-body">
<h1><img src="/assets/favicon.ico" width="{logo_d}" height="{logo_d}"></img> Note Viewer</h1>
The notes are separated into daily and paper-specific notes.
This page contains an overview over all present notes.
"""
        )

        file.write(
            f"""<h2>Paper-Notes</h2>
<input type="text" id="paper_search" onkeyup="filter('papers', 'paper_search')" placeholder="Search Tags or Names">
{filter_script}
<div style="height:50vh;width:100%;overflow:scroll;auto;padding-top:10px;">
<ul id="papers">
"""
        )

        for fname in sorted(f for f in files if f.endswith(".md") and "papers" in f):
            meta = get_paper_meta(fname)
            if meta is not None:
                try:
                    bibtex = Parser(meta["bibtex"]).parse()
                    if bibtex.is_err():
                        title = ""
                        authors = ""
                        year = ""
                    else:
                        assert isinstance(bibtex, Entry)
                        title = strip_value_or_empty(bibtex.get_or_none("title"))
                        authors = strip_value_or_empty(bibtex.get_or_none("author"))
                        year = strip_value_or_empty(bibtex.get_or_none("year")) + ": "

                    tags = ", ".join(meta["tags"])
                except KeyError:
                    tags = ""
                    title = ""
                    authors = ""
                    year = ""
            else:
                tags = ""
                title = ""
                authors = ""
                year = ""

            fpath = fname.replace(root_dir, "")
            fname = os.path.basename(fname).replace(".md", "")
            fpath = fpath.replace(".md", ".html")
            file.write(
                f'<li authors="{authors}" tags="{tags}" title="{title}"><strong>{title}</strong></br>{year}<em>{authors}</em></br><a href="{fpath}">{fname}</a></li>\n'
            )

        file.write("</ul>\n</div>")
        file.write(html_end)
        file.write(
            """<h2>Daily Notes</h2>
<div style="height:10vh;width:100%;overflow:scroll;auto;padding-top:10px;">
"""
        )

        file.write("<ul class='mcol_ul' id='papers_list'>\n")
        for fname in reversed(
            sorted(f for f in files if f.endswith(".md") and "daily" in f)
        ):
            fpath = fname.replace(root_dir, "")
            fname = os.path.basename(fname).replace(".md", "")
            fpath = fpath.replace(".md", ".html")
            file.write(f'<li class="mcol_li"><a href="{fpath}">{fname}</a></li>\n')
        file.write("</ul>\n</div>")

        file.write(
            """<h2>Other Notes</h2>
<div style="height:50vh;width:100%;overflow:scroll;auto;padding-top:10px;">
"""
        )
        file.write("<ul>\n")
        for fname in sorted(
            f
            for f in files
            if f.endswith(".md") and not "papers" in f and not "daily" in f
        ):
            fpath = fname.replace(root_dir, "")
            fname = os.path.basename(fname).replace(".md", "")
            fpath = fpath.replace(".md", ".html")
            file.write(f'<li><a href="{fpath}">{fname}</a></li>\n')
        file.write("</ul>\n</div>")


def convert_file(in_path, out_path, css_file_path, root_dir):
    logger.debug(f"converting {in_path} -> {out_path}")
    content = get_file_contents(in_path)
    if content is None:
        return

    logger.debug(f"{in_path}: valid content")
    match = header_re.match(content)
    html = html_start
    if match is not None:
        logger.debug(f"{in_path}: matched yaml header")
        header_len = len(match.group(0))
        match = match.group(1)
        header = yaml.load(match, Loader=yaml.CLoader)
        bibtex = Parser(header["bibtex"]).parse()
        if bibtex.is_err():
            logger.warn(f"{in_path}: invalid bibtex")
            title = ""
            authors = ""
            year = ""
        else:
            logger.debug(f"{in_path}: valid bibtex")
            assert isinstance(bibtex, Entry)
            title = strip_value_or_empty(bibtex.get_or_none("title"))
            authors = strip_value_or_empty(bibtex.get_or_none("author"))
            year = strip_value_or_empty(bibtex.get_or_none("year")) + ": "
        content = content[header_len:]
        logger.debug(f"found header {header}")
        try:
            pdf_name = os.path.basename(header["pdf"].replace(".pdf", ""))
        except KeyError:
            pdf_name = "<NONE>"
            logger.warn(f"Could not find pdf name in header for {in_path}")
        html += f"""
<title>{APP_NAME}: {title}</title>
<link rel="stylesheet" href="/{css_file_path}">
{markdown_insert}
</head>
<body class="markdown-body">
<a href="/index.html"><img src="/{assets_dir_name}/{favicon_file_name}" width="{int(logo_d / 2)}" height="{int(logo_d / 2)}"></img></a>
<a href=\"/papers/{header['pdf']}\">Note for {pdf_name}</a>
<p>{year}<em>{authors}</em></p>
"""
    else:
        name = in_path.replace(".md", "")
        name = os.path.relpath(name, start=root_dir)
        html += f"""
<title>{APP_NAME}: {name}</title>
<link rel="stylesheet" href="/{css_file_path}">
{markdown_insert}
</head>
<body class="markdown-body">
<a href="/index.html"><img src="/{assets_dir_name}/{favicon_file_name}" width="{logo_d}" height="{logo_d}"></img></a>
"""

    content = link_re.sub(r"(/\1.html\2)", content)
    content = pdf_re.sub(r"(/\1.pdf\2)", content)
    content = empty_re.sub(r"", content)
    replaces = [
        [r"\\", r"\\\\"],
        *[[f"\\{c}", f"\\\\{c}"] for c in "()[]{}"],
        *[[f"\\left{c}", f"\\left\\\\{c}"] for c in "{}"],
        *[[f"\\right{c}", f"\\right\\\\{c}"] for c in "{}"],
    ]

    for orig, repl in replaces:
        content = content.replace(orig, repl)

    converted = pycmarkgfm.gfm_to_html(
        content,
        options=pycmarkgfm.options.validate_utf8 | pycmarkgfm.options.unsafe,
    )

    with open(out_path, "w") as f:
        f.write(html)
        f.write(converted)
        f.write(html_end)


def refresh_files(serve_dir, root_dir):
    css_file_path = os.path.join(assets_dir_name, css_file_name)

    if os.path.exists(serve_dir):
        for elem in os.scandir(serve_dir):
            if elem.name == "assets":
                pass
            else:
                if elem.is_dir():
                    shutil.rmtree(elem.path)
                else:
                    os.remove(elem.path)
    else:
        os.mkdir(serve_dir)

    files, pdfs, folders = collect_structure(root_dir)

    # regenerate all folders
    for folder in folders:
        fpath = os.path.join(serve_dir, folder.replace(root_dir, "").strip("/"))
        os.mkdir(fpath)

    generate_index(serve_dir, css_file_path, files, root_dir)

    # copy css file to web/assets directory
    for pdf in pdfs:
        out_path = os.path.join(serve_dir, pdf.replace(root_dir, "").strip("/"))
        shutil.copyfile(pdf, out_path)

    for file in files:
        out_path = os.path.join(
            serve_dir, file.replace(root_dir, "").strip("/").replace(".md", ".html")
        )
        convert_file(file, out_path, css_file_path, root_dir)


class FileEventHandler(pyinotify.ProcessEvent):
    def __init__(self, cfg: Config):
        self.last_time = datetime.datetime.now()
        self.cfg = cfg

    def process_default(self, event):
        # TODO: refresh only the changed files

        if not (event.pathname.endswith(".md") or event.pathname.endswith(".pdf")):
            return

        if datetime.datetime.now() - self.last_time < datetime.timedelta(
            microseconds=100
        ):
            return

        logger.info(f"detected change at {event.pathname}. Regenerating...")
        assets_path = os.path.join(self.cfg.serve_path, assets_dir_name)
        refresh_files(self.cfg.serve_path, self.cfg.root_dir)
        self.last_time = datetime.datetime.now()

        window = subprocess.run(
            ["xdotool", "search", "--name", f"{APP_NAME}:"], capture_output=True
        )
        if window.returncode == 0:
            win_ids = window.stdout.decode().splitlines()
            for wid in win_ids:
                logger.debug(f"refreshing {wid}")
                subprocess.call(["xdotool", "key", "--window", wid, "F5"])


def get_args():
    parser = argparse.ArgumentParser(
        prog="envy.serve",
        description="""Serve a collection of markdown files in the browser""",
    )
    parser.add_argument(
        "-u", "--use-config", help="Path to configuration file", type=str, default=None
    )
    parser.add_argument(
        "-c", "--config-help", help="Show config file help", action="store_true"
    )
    parser.add_argument(
        "-d", "--default-config", help="Print the default config", action="store_true"
    )
    parser.add_argument(
        "-v",
        "--verbosity",
        choices=[l for l in logging._nameToLevel.keys()],
        default=logging._levelToName[logging.INFO],
    )
    parser.add_argument("-r", "--reload-assets", action="store_true")
    return parser.parse_args()


def main():
    args = get_args()
    logger.setLevel(args.verbosity)

    if args.config_help:
        print_config_help()
        exit(0)
    elif args.default_config:
        print(json.dumps(Config.DEFAULT, indent=4))
        exit(0)

    if args.use_config is not None:
        logger.debug(f"Reading config from {args.use_config}")
    else:
        logger.debug("Reading default config")
    cfg = get_config(fpath=args.use_config)

    if not os.path.exists(cfg.root_dir):
        logger.fatal(f"""Note directory {cfg.root_dir} does not exist. Exiting...""")
        exit(1)

    if not os.path.exists(cfg.serve_path):
        os.makedirs(cfg.serve_path)

    this_dir = os.path.abspath(os.path.dirname(__file__))
    favicon_orig_path = os.path.join(this_dir, assets_dir_name, favicon_file_name)
    favicon_dst_path = os.path.join(cfg.serve_path, assets_dir_name, favicon_file_name)
    assets_path = os.path.join(cfg.serve_path, assets_dir_name)

    if not os.path.exists(assets_path):
        os.makedirs(assets_path)
        download_assets(cfg)
        shutil.copyfile(favicon_orig_path, favicon_dst_path)
    else:
        if args.reload_assets:
            shutil.rmtree(assets_path)
            os.makedirs(assets_path)
            download_assets(cfg)
            shutil.copyfile(favicon_orig_path, favicon_dst_path)

    logger.info("Refreshing Files")
    refresh_files(cfg.serve_path, cfg.root_dir)

    wm = pyinotify.WatchManager()
    mask = pyinotify.IN_DELETE | pyinotify.IN_CLOSE_WRITE

    notifier = pyinotify.ThreadedNotifier(wm, FileEventHandler(cfg))
    notifier.start()
    watches = [cfg.root_dir]
    for watch in watches:
        wdd = wm.add_watch(watch, mask, rec=True)
        if wdd[watch] > 0:
            logger.info(f'watching "{watch}" for changes...')
        else:
            logger.warn(f"Error watching {watch}.")

    logger.info(f"Running at http://{ADDRESS}:{PORT}")

    class Handler(SimpleHTTPRequestHandler):
        def __init__(self, *args, **kwargs):
            super().__init__(*args, directory=cfg.serve_path, **kwargs)

    logger.debug(f"Serving directory {cfg.serve_path}")
    server = HTTPServer((ADDRESS, PORT), Handler)
    server.serve_forever()


if __name__ == "__main__":
    main()
