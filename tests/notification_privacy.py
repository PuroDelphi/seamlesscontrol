"""Verify file metadata reaches notifications over D-Bus without entering argv."""

import json
import os
import subprocess
import sys
import tempfile
import threading
from pathlib import Path

import gi

gi.require_version("Gio", "2.0")
from gi.repository import Gio, GLib  # noqa: E402

SERVICE = "org.freedesktop.Notifications"
OBJECT = "/org/freedesktop/Notifications"
SECRET = "private-review-file.txt"
INTERFACE_XML = """<node><interface name="org.freedesktop.Notifications">
  <method name="Notify">
    <arg type="s" direction="in"/><arg type="u" direction="in"/>
    <arg type="s" direction="in"/><arg type="s" direction="in"/>
    <arg type="s" direction="in"/><arg type="as" direction="in"/>
    <arg type="a{sv}" direction="in"/><arg type="i" direction="in"/>
    <arg type="u" direction="out"/>
  </method>
  <signal name="ActionInvoked"><arg type="u"/><arg type="s"/></signal>
</interface></node>"""


def start_mock_server():
    connection = Gio.bus_get_sync(Gio.BusType.SESSION, None)
    request = GLib.Variant("(su)", (SERVICE, 0))
    result = connection.call_sync(
        "org.freedesktop.DBus", "/org/freedesktop/DBus",
        "org.freedesktop.DBus", "RequestName", request,
        GLib.VariantType("(u)"), Gio.DBusCallFlags.NONE, -1, None,
    )
    assert result.unpack()[0] == 1
    received = []

    def on_method(conn, _sender, path, interface_name, method, params, invocation):
        assert method == "Notify"
        received.append(params.unpack())
        invocation.return_value(GLib.Variant("(u)", (42,)))

        def reject():
            conn.emit_signal(
                None, path, interface_name, "ActionInvoked",
                GLib.Variant("(us)", (42, "reject")),
            )
            return False

        GLib.timeout_add(50, reject)

    interface = Gio.DBusNodeInfo.new_for_xml(INTERFACE_XML).interfaces[0]
    connection.register_object(OBJECT, interface, on_method, None, None)
    loop = GLib.MainLoop()
    threading.Thread(target=loop.run, daemon=True).start()
    return connection, loop, received


def invoke_without_argv_secret(binary, arguments, payload, env=None):
    process = subprocess.Popen(
        [binary, *arguments], stdin=subprocess.PIPE,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=env,
    )
    argv = Path(f"/proc/{process.pid}/cmdline").read_bytes()
    assert SECRET.encode() not in argv, argv
    output, error = process.communicate((json.dumps(payload) + "\n").encode(), timeout=10)
    return process.returncode, output, error


def main(binary):
    _connection, loop, received = start_mock_server()
    offer = {
        "summary": "Incoming file",
        "body": f"{SECRET} · 192.0.2.7 · 123 bytes",
        "accept": "Accept",
        "reject": "Reject",
    }
    try:
        code, output, error = invoke_without_argv_secret(binary, ["notify-file-offer"], offer)
    finally:
        loop.quit()
    assert code == 0, error.decode()
    assert output.strip() == b"reject", (output, error)
    assert received and SECRET in received[0][4], received
    assert received[0][5] == ["accept", "Accept", "reject", "Reject"]
    print("Private D-Bus notification delivered; action returned; filename absent from argv.")

    with tempfile.TemporaryDirectory(prefix="seamlesscontrol-ui-stdin-") as work:
        source = Path(work) / SECRET
        source.write_text("private test content")
        env = os.environ.copy()
        env.update(XDG_CACHE_HOME=work, XDG_CONFIG_HOME=work)
        for arguments, payload in [
            (["prepare-dropped-files-ui-stdin"], [source.as_uri()]),
            (["send-file-ui-stdin", "127.0.0.1:1"], str(source)),
        ]:
            code, output, error = invoke_without_argv_secret(binary, arguments, payload, env)
            if arguments[0].startswith("prepare"):
                assert code == 0, error.decode()
                assert Path(json.loads(output.decode())).is_file()
            else:
                assert code != 0 and SECRET.encode() not in error, (output, error)
    print("UI file paths reached the agent through stdin and stayed out of argv.")


if __name__ == "__main__":
    main(sys.argv[1])
