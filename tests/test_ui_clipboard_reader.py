"""The panel's explicit paste must never buffer an unbounded Wayland offer."""

import json
import os
from pathlib import Path
import subprocess
import tempfile
import time
import unittest


READER = Path(__file__).resolve().parents[1] / "packaging/read-ui-clipboard.py"


class ClipboardReaderTest(unittest.TestCase):
    def run_reader(self, producer):
        with tempfile.TemporaryDirectory() as directory:
            stub = Path(directory) / "wl-paste"
            stub.write_text("#!/usr/bin/env python3\n" + producer)
            stub.chmod(0o755)
            env = dict(os.environ, PATH=directory + os.pathsep + os.environ["PATH"])
            started = time.monotonic()
            result = subprocess.run(
                ["python3", str(READER)], env=env, text=True,
                capture_output=True, timeout=5, check=True,
            )
            return result.stdout, time.monotonic() - started

    def test_small_unicode_selection(self):
        output, _ = self.run_reader("import sys\nsys.stdout.write('/home/área/漢字.txt')\n")
        self.assertEqual(json.loads(output), {"text": "/home/área/漢字.txt"})

    def test_large_selection_is_discarded(self):
        output, _ = self.run_reader("import os\nos.write(1, b'x' * 100000)\n")
        self.assertEqual(output, "")

    def test_stalled_selection_times_out(self):
        output, elapsed = self.run_reader("import time\ntime.sleep(10)\n")
        self.assertEqual(output, "")
        self.assertLess(elapsed, 4)

    def test_multiline_selection_is_discarded(self):
        output, _ = self.run_reader("import sys\nsys.stdout.write('one\\ntwo')\n")
        self.assertEqual(output, "")

    def test_invalid_utf8_is_discarded(self):
        output, _ = self.run_reader("import os\nos.write(1, b'\\xff')\n")
        self.assertEqual(output, "")


if __name__ == "__main__":
    unittest.main()
