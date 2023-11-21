#!/usr/bin/env python3
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


logger = logging.Logger("logger")
hn = logging.StreamHandler()
hn.setFormatter(logging.Formatter("%(asctime)s %(message)s"))
logger.addHandler(hn)
logger.setLevel(logging.INFO)


link_re = re.compile(r"\((.*)\.md\)")
header_re = re.compile(r"---\n([\s\S]*)\n---\n", flags=re.MULTILINE)
serve_path = "./web"
css_file_name = "./assets/github-markdown-dark.css"
html_start = "<!DOCTYPE html>\n<html>\n"
html_end = "</body>\n</html>"
PORT = 6969
ADDRESS = "localhost"
APP_NAME = "NoteView"


class Handler(SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=serve_path, **kwargs)


def collect_structure(folder_path) -> tuple[list[str], list[str], list[str]]:
    files = []
    folders = []
    pdfs = []
    for ent in os.scandir(folder_path):
        if ent.is_file():
            if ent.path.endswith(".md"):
                files.append(ent.path)
            elif ent.path.endswith(".pdf"):
                pdfs.append(ent.path)
        elif ent.is_dir():
            folders.append(ent.path)
            sub_files, sub_pdfs, sub_folders = collect_structure(ent.path)
            files.extend(sub_files)
            folders.extend(sub_folders)
            pdfs.extend(sub_pdfs)
        else:
            logger.debug(f"Ignoring {ent.path}")

    return (
        files,
        pdfs,
        folders,
    )


def refresh_files():
    if os.path.exists(serve_path):
        shutil.rmtree(serve_path)

    files, pdfs, folders = collect_structure(".")
    os.mkdir(serve_path)
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

    shutil.copyfile(css_file_name, os.path.join(serve_path, css_file_name))
    with open(os.path.join(serve_path, "index.html"), "w") as file:
        file.write(
            f"""{html_start}
<meta name="viewport" content="width=device-width, initial-scale=1">
<link rel="stylesheet" href="/{css_file_name}">
{markdown_insert}
<title>{APP_NAME}: Collection of my Notes</title>
</head>
<body class="markdown-body">
<h1>Collection of my Notes</h1>
The notes are separated into daily and paper-specific notes.
This page contains an overview over all present notes.
<h2>Paper-Notes</h2>
<ul>
"""
        )
        for fname in sorted(f for f in files if f.endswith(".md") and "papers" in f):
            fpath = fname
            fname = os.path.basename(fname).replace(".md", "")
            fpath = fpath.replace(".md", ".html")
            file.write(f'<li><a href="{fpath}">{fname}</a></li>\n')

        file.write("</ul>\n")
        file.write(html_end)
        file.write("<h2>Daily Notes</h2>")

        file.write("<ul>\n")
        for fname in reversed(
            sorted(f for f in files if f.endswith(".md") and "daily" in f)
        ):
            fpath = fname
            fname = os.path.basename(fname).replace(".md", "")
            fpath = fpath.replace(".md", ".html")
            file.write(f'<li><a href="{fpath}">{fname}</a></li>\n')
        file.write("</ul>\n")

    for folder in folders:
        fpath = os.path.join(serve_path, folder)
        os.mkdir(fpath)

    for pdf in pdfs:
        shutil.copyfile(pdf, os.path.join(serve_path, pdf))

    for file in files:
        out_path = os.path.join(serve_path, file.replace(".md", ".html"))
        logger.debug(f"converting {file} -> {out_path}")
        with open(file, "r") as f:
            content = f.read()
            match = header_re.match(content)
            html = html_start
            if match is not None:
                header_len = len(match.group(0))
                match = match.group(1)
                header = yaml.load(match, Loader=yaml.CLoader)
                content = content[header_len:]
                logger.debug(f"found header {header}")
                pdf_name = os.path.basename(header["pdf"].replace(".pdf", ""))
                html += f"""<head>
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{APP_NAME}: {header['title']}</title>
<link rel="stylesheet" href="/{css_file_name}">
{markdown_insert}
</head>
<body class="markdown-body">
<a href=\"/papers/{header['pdf']}\">Note for {pdf_name}</a>
"""
            else:
                name = file.replace(".md", "")
                html += f"""<head>
<title>{APP_NAME}: {name}</title>
<meta name="viewport" content="width=device-width, initial-scale=1">
<link rel="stylesheet" href="/{css_file_name}">
{markdown_insert}
</head>
<body class="markdown-body">
"""
            content = link_re.sub(r"(/\1.html)", content)
            converted = pycmarkgfm.gfm_to_html(content)

        with open(out_path, "w") as f:
            f.write(html)
            f.write(converted)
            f.write(html_end)

server = HTTPServer((ADDRESS, PORT), Handler)

class FileEventHandler(pyinotify.ProcessEvent):
    def __init__(self):
        self.last_time = datetime.datetime.now()


    def process_default(self, event):
        # TODO: refresh only the changed files

        if not (event.pathname.endswith(".md") or event.pathname.endswith(".pdf")):
            return
        
        if datetime.datetime.now() - self.last_time < datetime.timedelta(microseconds=100):
            return

        logger.info(f"detected change at {event.pathname}. Regenerating...")
        refresh_files()
        self.last_time = datetime.datetime.now()

        window = subprocess.run(["xdotool", "search", "--name", "NoteView: "], capture_output=True)
        if window.returncode == 0:
            win_id = window.stdout
            logger.debug(f"refreshing {win_id}")
            subprocess.call(["xdotool", "key", "--window", win_id, "F5"])



def main():
    logger.info("Refreshing Files")
    refresh_files()

    wm = pyinotify.WatchManager()
    mask = pyinotify.IN_DELETE | pyinotify.IN_CLOSE_WRITE

    notifier = pyinotify.ThreadedNotifier(wm, FileEventHandler())
    notifier.start()
    watches = ["./daily", "./papers", "./notes.md"]
    for watch in watches:
        wdd = wm.add_watch(watch, mask, rec=True)
        if wdd[watch] > 0:
            logger.info(f"watching \"{watch}\" for changes...")
        else:
            logger.warn(f"Error watching {watch}.")

    logger.info(f"Running at http://{ADDRESS}:{PORT}")
    server.serve_forever()


if __name__ == "__main__":
    main()
