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

link_re = re.compile(r"\(notes/(.*?)\.md\)")
pdf_re = re.compile(r"\(notes/(.*?).pdf(#.*){0,1}\)")
empty_re = re.compile(r"\[next\]\(<empty>\)")
header_re = re.compile(r"---\n([\s\S]*)\n---\n", flags=re.MULTILINE)
base_path = "./notes/"

APP_NAME = "NoteView"
ADDRESS = "localhost"
PORT = 6969

html_start = """<!DOCTYPE html>
<html>
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <meta charset="utf-8">
    <link rel="icon" type="image/x-icon" href="/favicon.ico">
    <script async src="/assets/mathjax/tex-chtml.js" id="MathJax-script"></script>"""
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
filter_script = """
<script>
function matches(tags, filter) {
  if (filter == 'READ' || filter == 'UNREAD' || filter == 'READING') {
    return tags.map((tag) => tag.toUpperCase()).includes(filter);
  }

  for (let i = 0; i < tags.length; i++) {
    if (tags[i].toUpperCase().includes(filter)) {
      return true;
    }
  }
  return false;
}

function filter(list_id, query_id) {
  var input = document.getElementById(query_id);
  var filter = input.value.toUpperCase();
  var ul = document.getElementById(list_id);
  li = ul.getElementsByTagName('li');

  if (filter == "") {
      return;
  }
  var a;
  for (let i = 0; i < li.length; ++i) {
    a = li[i].getElementsByTagName('a')[0];
    var val = (a.textContent || a.innerText).toUpperCase();
    var tags = li[i].getAttribute("tags").split(", ");

    if (val.includes(filter) || matches(tags, filter)) {
      li[i].style.display = "";
    } else {
      li[i].style.display = "none";
    }
  }
}
</script>
"""

logo_d = 32

serve_path = "web"
assets_path = "assets"
favicon_path = "favicon.ico"
css_file_name = "github-markdown-dark.css"


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


def get_paper_meta(in_path):
    with open(in_path, "r") as f:
        content = f.read()
        match = header_re.match(content)
        if match is not None:
            header = yaml.load(match.group(1), Loader=yaml.CLoader)
            return header

        return None


def generate_index(css_file_path, files):
    # TODO: generate search window based on tags
    with open(os.path.join(serve_path, "index.html"), "w") as file:
        file.write(
            f"""{html_start}
<link rel="stylesheet" href="/{css_file_path}">
{markdown_insert}
<title>{APP_NAME}: Collection of my Notes</title>
</head>
<body class="markdown-body">
<h1><img src="/favicon.ico" width="{logo_d}" height="{logo_d}"></img>oteView: Collection of my Notes</h1>
The notes are separated into daily and paper-specific notes.
This page contains an overview over all present notes.
"""
        )

        file.write(f"""<h2>Paper-Notes</h2>
<input type="text" id="paper_search" onkeyup="filter('papers', 'paper_search')" placeholder="Search Tags or Names">
{filter_script}
<ul id="papers">
""")

        for fname in sorted(f for f in files if f.endswith(".md") and "papers" in f):
            meta = get_paper_meta(fname)
            if meta is not None:
                try:
                    tags = ", ".join(meta["tags"])
                except KeyError:
                    tags = ""
            else:
                tags = ""

            fpath = fname.replace(base_path, "")
            fname = os.path.basename(fname).replace(".md", "")
            fpath = fpath.replace(".md", ".html")
            file.write(f'<li tags="{tags}"><a href="{fpath}">{fname}</a></li>\n')

        file.write("</ul>\n")
        file.write(html_end)
        file.write("<h2>Daily Notes</h2>")

        file.write("<ul id='papers_list'>\n")
        for fname in reversed(
            sorted(f for f in files if f.endswith(".md") and "daily" in f)
        ):
            fpath = fname.replace(base_path, "")
            fname = os.path.basename(fname).replace(".md", "")
            fpath = fpath.replace(".md", ".html")
            file.write(f'<li><a href="{fpath}">{fname}</a></li>\n')
        file.write("</ul>\n")

        file.write("<h2>Other Notes</h2>\n")
        file.write("<ul>\n")
        for fname in sorted(f for f in files if f.endswith(".md") and not "papers" in f and not "daily" in f):
            fpath = fname.replace(base_path, "")
            fname = os.path.basename(fname).replace(".md", "")
            fpath = fpath.replace(".md", ".html")
            file.write(f'<li><a href="{fpath}">{fname}</a></li>\n')
        file.write("</ul>\n")




def convert_file(in_path, out_path, css_file_path):
    logger.debug(f"converting {in_path} -> {out_path}")
    with open(in_path, "r") as f:
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
            html += f"""
<title>{APP_NAME}: {header['title']}</title>
<link rel="stylesheet" href="/{css_file_path}">
{markdown_insert}
</head>
<body class="markdown-body">
<a href="/index.html"><img src="/favicon.ico" width="{int(logo_d / 2)}" height="{int(logo_d / 2)}"></img></a>
<a href=\"/papers/{header['pdf']}\">Note for {pdf_name}</a>
"""
        else:
            name = in_path.replace(".md", "")
            html += f"""
<title>{APP_NAME}: {name}</title>
<link rel="stylesheet" href="/{css_file_path}">
{markdown_insert}
</head>
<body class="markdown-body">
<a href="/index.html"><img src="/favicon.ico" width="{logo_d}" height="{logo_d}"></img></a>
"""

        content = link_re.sub(r"(/\1.html)", content)
        content = pdf_re.sub(r"(/\1.pdf\2)", content)
        content = empty_re.sub(r"", content)
        content = content.replace(r"\(", r"\\(")
        content = content.replace(r"\)", r"\\)")
        content = content.replace(r"\[", r"\\[")
        content = content.replace(r"\]", r"\\]")
        content = content.replace(r"\{", r"\\{")
        content = content.replace(r"\}", r"\\}")
        converted = pycmarkgfm.gfm_to_html(content, options=pycmarkgfm.options.validate_utf8)

        with open(out_path, "w") as f:
            f.write(html)
            f.write(converted)
            f.write(html_end)


def refresh_files():
    css_file_path = os.path.join(assets_path, css_file_name)

    if os.path.exists(serve_path):
        shutil.rmtree(serve_path)

    files, pdfs, folders = collect_structure(base_path)

    # regenerate all folders
    os.mkdir(serve_path)
    os.mkdir(os.path.join(serve_path, assets_path))
    for folder in folders:
        fpath = os.path.join(serve_path, folder.replace(base_path, ""))
        os.mkdir(fpath)

    generate_index(css_file_path, files)

    # copy css file to assets
    shutil.copyfile(css_file_path, os.path.join(serve_path, css_file_path))
    shutil.copyfile(favicon_path, os.path.join(serve_path, favicon_path))
    shutil.copytree("./assets/mathjax/es5", os.path.join(serve_path, assets_path, "mathjax"))

    for pdf in pdfs:
        out_path = os.path.join(serve_path, pdf.replace(base_path, ""))
        shutil.copyfile(pdf, out_path)

    for file in files:
        out_path = os.path.join(
            serve_path, file.replace(base_path, "").replace(".md", ".html")
        )
        convert_file(file, out_path, css_file_path)



server = HTTPServer((ADDRESS, PORT), Handler)


class FileEventHandler(pyinotify.ProcessEvent):
    def __init__(self):
        self.last_time = datetime.datetime.now()

    def process_default(self, event):
        # TODO: refresh only the changed files

        if not (event.pathname.endswith(".md") or event.pathname.endswith(".pdf")):
            return

        if datetime.datetime.now() - self.last_time < datetime.timedelta(
            microseconds=100
        ):
            return

        logger.info(f"detected change at {event.pathname}. Regenerating...")
        refresh_files()
        self.last_time = datetime.datetime.now()

        window = subprocess.run(
            ["xdotool", "search", "--name", "NoteView:"],
            capture_output=True
        )
        if window.returncode == 0:
            win_ids = window.stdout.decode().splitlines()
            for wid in win_ids:
                logger.debug(f"refreshing {wid}")
                subprocess.call(["xdotool", "key", "--window", wid, "F5"])


def main():
    try:
        log_environ = os.environ['LOG_LEVEL']
    except KeyError:
        log_environ = None
    
    if log_environ is None:
        logger.setLevel(logging.INFO)
    else:
        try:
            level = logging._nameToLevel[log_environ]
            logger.setLevel(level)
        except KeyError:
            logger.setLevel(logging.INFO)
            logger.warn(f"Provided invalid log level {log_environ}. Valid levels are {[l for l in logging._nameToLevel.keys()]}")


    logger.info("Refreshing Files")
    refresh_files()

    wm = pyinotify.WatchManager()
    mask = pyinotify.IN_DELETE | pyinotify.IN_CLOSE_WRITE

    notifier = pyinotify.ThreadedNotifier(wm, FileEventHandler())
    notifier.start()
    watches = ["./notes"]
    for watch in watches:
        wdd = wm.add_watch(watch, mask, rec=True)
        if wdd[watch] > 0:
            logger.info(f'watching "{watch}" for changes...')
        else:
            logger.warn(f"Error watching {watch}.")

    logger.info(f"Running at http://{ADDRESS}:{PORT}")
    server.serve_forever()


if __name__ == "__main__":
    main()
