#!/usr/bin/env python3
"""Render the real Windows interface with fictional documentation data.

Run with no arguments for all bilingual images, or pass view names to refresh a
subset. Requires Chromium/Google Chrome; no Windows agent or network is started.
The injected states are fictional documentation examples. Focused views only
hide surrounding interface sections; they use the same HTML and render functions.
"""

import html as html_module
import json
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile


ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "agent/src/windows_ui.html"
OUTPUT = ROOT / "docs/images"
VIEWS = {
    "home": ("home", 1200),
    "devices": ("devices", 1200),
    "files": ("files", 1200),
    "settings": ("settings", 1200),
    "pairing": ("devices", 960),
    "layout": ("devices", 1060),
    "offer": ("files", 960),
}
FINGERPRINT = "0123456789abcdef0123456789abcdef"
FOCUS_CSS = {
    "pairing": "#view-devices > :not(#pairFeedback):not(#pairBox){display:none}",
    "layout": (
        "#view-devices > :not(#peers):not(#layoutGuide):not(.layout-panel)"
        ":not(.section-head:has(+ #peers))"
        ":not(.section-head:has(+ .layout-panel)){display:none}"
    ),
    "offer": "#view-files > :not(article:first-of-type){display:none}",
}


def fixture(language: str, view: str) -> dict:
    """Supply the UI's normal state shape, never a successful live operation."""
    state = {
        "receive": True, "connect": False, "fileReceive": False,
        "controlMode": "receive", "receivePort": 47832,
        "lastAddress": "192.0.2.20:47832", "lastEdge": "right",
        "clipboardReady": True, "copiedFile": "example-notes.txt",
        "clipboardOffer": "", "clipboardProgress": None,
        "peers": [{"ip": "192.0.2.20", "fingerprint": FINGERPRINT}],
        "discovered": [{
            "name": "Omarchy (example)" if language == "en" else "Omarchy (ejemplo)",
            "address": "192.0.2.42:47832",
            "fingerprint": "fedcba9876543210fedcba9876543210", "trust": "new",
        }],
        "layout": {FINGERPRINT: "right"}, "logs": [],
        "fileLimitMiB": 100, "approvalMode": "always", "approvalMinutes": 15,
        "approvalFeedback": "", "approvalSecondsRemaining": 0, "startup": "off",
        "defaultDownload": r"C:\Users\Example\Downloads",
        "message": "",
    }
    if view == "pairing":
        state.update(pairCode="482731", pairStatus="code", peers=[], layout={})
    if view == "offer":
        state.update(
            copiedFile=None,
            clipboardOffer="OFFER\t192.0.2.20\texample-notes.txt\t2048\texample-digest",
        )
    return state


def render(chromium: str, language: str, view: str, directory: Path) -> None:
    tab_name, width = VIEWS[view]
    page_html = SOURCE.read_text(encoding="utf-8")
    page_html = page_html.replace("192.168.1.25", "192.0.2.20")
    # The real page posts initial IPC messages. No backend exists in this render.
    page_html = page_html.replace(
        "<script>", "<script>window.ipc={postMessage:()=>{}};</script><script>", 1
    )
    provenance = (
        "SeamlessControl for Windows · fictional example data"
        if language == "en" else
        "SeamlessControl para Windows · datos de ejemplo ficticios"
    )
    css = (
        ".doc-provenance{padding:12px 20px;background:#243644;color:#f1eee5;"
        "font:14px/1.5 sans-serif;border-bottom:1px solid #90c7cd}"
        ".main{max-height:none;overflow:visible}.app{min-height:0}"
    )
    if view in FOCUS_CSS:
        css += (
            ".sidebar,.topbar,.footer{display:none}.app{display:block}"
            ".content{padding:24px;max-width:none}"
        ) + FOCUS_CSS[view]
    page_html = page_html.replace("</head>", f"<style>{css}</style></head>")
    page_html = page_html.replace(
        "<body>", f'<body><div class="doc-provenance">{html_module.escape(provenance)}</div>', 1
    )
    injected = (
        "<script>\n"
        f"language={json.dumps(language)};applyLanguage();\n"
        f"window.seamlessReceive({json.dumps(fixture(language, view))});\n"
        f"tab({json.dumps(tab_name)});\n"
        + (f"selectedPeer={json.dumps(FINGERPRINT)};lastDevices='';renderDevices();\n"
           if view == "layout" else "")
        + "document.body.dataset.docHeight=String(Math.ceil("
        "document.querySelector('.app').getBoundingClientRect().height+"
        "document.querySelector('.doc-provenance').getBoundingClientRect().height));\n"
        "</script></body>"
    )
    page_html = page_html.replace("</body>", injected)
    page = directory / f"windows-{view}-{language}.html"
    page.write_text(page_html, encoding="utf-8")
    common = [
        chromium, "--headless", "--no-sandbox", "--disable-gpu",
        "--hide-scrollbars", "--force-device-scale-factor=1",
        "--disable-background-networking", "--no-first-run",
        f"--user-data-dir={directory / ('profile-' + view + '-' + language)}",
    ]
    # Size to the real content instead of clipping the lower cards or map.
    dom = subprocess.run(
        common + [f"--window-size={width},900", "--dump-dom", page.as_uri()],
        check=True, capture_output=True, text=True,
    ).stdout
    match = re.search(r'data-doc-height="(\d+)"', dom)
    if not match:
        raise RuntimeError(f"The {view}/{language} page did not render its fixture")
    height = int(match.group(1)) + 16
    subprocess.run(
        common + [f"--window-size={width},{height}",
                  f"--screenshot={OUTPUT / ('windows-' + view + '-' + language + '.png')}",
                  page.as_uri()],
        check=True,
    )


def main() -> None:
    chromium = shutil.which("chromium") or shutil.which("google-chrome")
    if not chromium:
        raise SystemExit("Chromium is required to render the documentation illustrations")
    views = tuple(sys.argv[1:]) or tuple(VIEWS)
    if any(view not in VIEWS for view in views):
        raise SystemExit("Views: " + ", ".join(VIEWS))
    OUTPUT.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="seamlesscontrol-doc-") as temp:
        directory = Path(temp)
        for language in ("en", "es"):
            for view in views:
                render(chromium, language, view, directory)


if __name__ == "__main__":
    main()
