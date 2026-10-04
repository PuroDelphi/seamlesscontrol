#!/usr/bin/env python3
"""Render the Windows app's real HTML with fictional data for documentation."""

from pathlib import Path
import shutil
import subprocess
import tempfile


ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "agent/src/windows_ui.html"
OUTPUT = ROOT / "docs/images"


def render(chromium: str, language: str, view: str, directory: Path) -> None:
    html = SOURCE.read_text()
    html = html.replace("192.168.1.25", "192.0.2.20")
    html = html.replace("<script>", "<script>window.ipc={postMessage:()=>{}};</script><script>", 1)
    fixture = """<script>
language = '__LANG__';
applyLanguage();
window.seamlessReceive({
  receive: true, connect: false, fileReceive: false,
  clipboardReady: true, copiedFile: 'example-notes.txt',
  clipboardOffer: '', clipboardProgress: null,
  peers: [{ip:'192.0.2.20', fingerprint:'0123456789abcdef0123456789abcdef'}],
  discovered: [], layout: {}, logs: [], fileLimitMiB: 100,
  approvalMode: 'timed', approvalMinutes: 15,
  defaultDownload: 'Downloads'
});
tab('__VIEW__');
</script></body>""".replace("__LANG__", language).replace("__VIEW__", view)
    html = html.replace("</body>", fixture)
    page = directory / f"windows-{view}-{language}.html"
    page.write_text(html)
    output = OUTPUT / f"windows-{view}-{language}.png"
    subprocess.run(
        [
            chromium, "--headless", "--no-sandbox", "--disable-gpu",
            "--hide-scrollbars", f"--user-data-dir={directory / 'profile'}",
            "--window-size=1440,1200", f"--screenshot={output}", page.as_uri(),
        ],
        check=True,
    )


def main() -> None:
    chromium = shutil.which("chromium") or shutil.which("google-chrome")
    if not chromium:
        raise SystemExit("Chromium is required to render the documentation screenshots")
    with tempfile.TemporaryDirectory(prefix="seamlesscontrol-doc-") as temp:
        directory = Path(temp)
        for language in ("en", "es"):
            for view in ("files", "settings"):
                render(chromium, language, view, directory)


if __name__ == "__main__":
    main()
