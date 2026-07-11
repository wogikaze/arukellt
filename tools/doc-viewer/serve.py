#!/usr/bin/env python3
"""Arukellt doc viewer — local SPA server.

Serves the self-contained SPA at tools/doc-viewer/ and maps /docs/* to the
repository docs/ directory, so the SPA can fetch markdown via same-origin
requests with zero CDN dependencies.

Usage:
    python3 tools/doc-viewer/serve.py [-p PORT] [--no-open]

Routes:
    /                       -> tools/doc-viewer/index.html
    /app.js, /styles.css    -> tools/doc-viewer/...
    /vendor/*               -> tools/doc-viewer/vendor/...
    /docs/*                 -> <repo>/docs/...
    /docs/_sidebar.md       -> <repo>/docs/_sidebar.md
"""
import argparse
import http.server
import os
import socketserver
import sys
import webbrowser

REPO_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
VIEWER_DIR = os.path.join(REPO_ROOT, "tools", "doc-viewer")
DOCS_DIR = os.path.join(REPO_ROOT, "docs")


class ViewerHandler(http.server.SimpleHTTPRequestHandler):
    """Map /docs/* to repo docs/, everything else to viewer dir."""

    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=VIEWER_DIR, **kwargs)

    def translate_path(self, path):
        # Strip query string.
        path = path.split("?", 1)[0].split("#", 1)[0]
        if path.startswith("/docs/"):
            rel = path[len("/docs/"):]
            return os.path.normpath(os.path.join(DOCS_DIR, rel))
        if path == "/docs":
            return DOCS_DIR
        return super().translate_path(path)

    def end_headers(self):
        # Disable caching for markdown so edits show on refresh.
        if self.path.endswith(".md"):
            self.send_header("Cache-Control", "no-store")
        super().end_headers()

    def log_message(self, fmt, *args):
        # Quieter log: just method + path + code.
        sys.stderr.write("%s - %s\n" % (self.address_string(), fmt % args))


def find_free_port(start, attempts=10):
    for _ in range(attempts):
        try:
            with socketserver.TCPServer(("127.0.0.1", start), None):
                pass
            return start
        except OSError:
            start += 1
    return start


def open_browser(url):
    for cmd in (["xdg-open", url], ["wslview", url]):
        try:
            import subprocess
            subprocess.Popen(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            return
        except FileNotFoundError:
            continue
    if os.environ.get("WSL_DISTRO_NAME"):
        try:
            import subprocess
            subprocess.Popen(["cmd.exe", "/c", "start", "", url],
                             stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        except Exception:
            pass
        return
    try:
        webbrowser.open(url)
    except Exception:
        pass


def main():
    ap = argparse.ArgumentParser(description="Arukellt doc viewer SPA server")
    ap.add_argument("-p", "--port", type=int, default=8765)
    ap.add_argument("--no-open", action="store_true", help="do not open browser")
    args = ap.parse_args()

    if not os.path.isfile(os.path.join(VIEWER_DIR, "index.html")):
        print("serve.py: viewer index.html not found at %s" % VIEWER_DIR, file=sys.stderr)
        return 1
    if not os.path.isdir(DOCS_DIR):
        print("serve.py: docs dir not found at %s" % DOCS_DIR, file=sys.stderr)
        return 1

    port = find_free_port(args.port)
    url = "http://127.0.0.1:%d/" % port
    print("serve.py: viewer at %s" % url)
    print("serve.py: docs served from %s" % DOCS_DIR)
    print("serve.py: press Ctrl-C to stop")

    if not args.no_open:
        open_browser(url)

    handler = ViewerHandler
    httpd = socketserver.TCPServer(("127.0.0.1", port), handler)
    try:
        httpd.serve_forever()
    except KeyboardInterrupt:
        print("\nserve.py: stopped")
        httpd.shutdown()


if __name__ == "__main__":
    sys.exit(main() or 0)
