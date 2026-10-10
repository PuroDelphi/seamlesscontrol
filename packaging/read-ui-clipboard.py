#!/usr/bin/env python3
"""Read a small text selection outside Quickshell, with byte and time bounds.

This is used only for an explicit paste in a SeamlessControl settings field.
Qt's editable TextInput clipboard watcher must not run in the shell process.
"""

import json
import os
import resource
import selectors
import subprocess
import sys
import time


MAX_BYTES = 4096
TIMEOUT_SECONDS = 2.0


def limit_child_memory():
    resource.setrlimit(resource.RLIMIT_AS, (256 * 1024 * 1024,) * 2)


def read_text():
    child = subprocess.Popen(
        ["wl-paste", "--no-newline"],
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        preexec_fn=limit_child_memory,
    )
    selector = selectors.DefaultSelector()
    selector.register(child.stdout, selectors.EVENT_READ)
    deadline = time.monotonic() + TIMEOUT_SECONDS
    data = bytearray()
    try:
        while True:
            remaining = deadline - time.monotonic()
            if remaining <= 0 or not selector.select(remaining):
                return None
            chunk = os.read(child.stdout.fileno(), MAX_BYTES + 1 - len(data))
            if not chunk:
                break
            data.extend(chunk)
            if len(data) > MAX_BYTES:
                return None
        if child.wait(timeout=max(0.01, deadline - time.monotonic())) != 0:
            return None
        value = data.decode("utf-8").rstrip("\r\n")
        if any(ord(char) < 32 for char in value):
            return None
        return value
    except (UnicodeDecodeError, subprocess.TimeoutExpired):
        return None
    finally:
        selector.close()
        if child.poll() is None:
            child.kill()
            child.wait()


def main():
    try:
        value = read_text()
    except (OSError, ValueError):
        value = None
    if value is not None:
        json.dump({"text": value}, sys.stdout, ensure_ascii=False)


if __name__ == "__main__":
    main()
