#!/usr/bin/env python3
"""Render the real plugin QML with fictional state, never the live agent.

Requires Python 3, quickshell and the installed Omarchy shell UI components.
Usage: python3 scripts/render-omarchy-doc-screenshots.py [--output DIR]

The original panel, translations, buttons and help rows are copied unchanged
into a temporary shell. Only KeyboardPanel (the layer-shell window container)
is replaced by an offscreen Item. AgentBackend is NOT instantiated. Its property
names are used to create a read-only documentation fixture, with no networking,
clipboard monitoring, installation, firewall or input-capture processes.
Each image carries an explicit fictional-data/offscreen-render label.
"""
import argparse
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]

SURFACE = '''import QtQuick
import qs.Commons
Item {
  property var anchorItem: null
  property var owner: null
  property var bar: null
  property bool open: false
  property var focusTarget: null
  property int contentWidth: 430
  property int contentHeight: 900
  width: contentWidth
  height: contentHeight
  function fittedContentWidth(value) { return value }
  function fittedContentHeight(value) { return Math.min(value, 1500) }
}
'''

SHELL = '''import QtQuick
import QtQuick.Window
import QtQuick.Controls as Controls
import Quickshell
import qs.Commons
import "plugin" as Plugin
ShellRoot {
  Controls.ApplicationWindow {
    id: window
    visible: true
    width: 478
    height: 1560
    color: "#101315"
    palette.text: "#cacccc"
    palette.placeholderText: "#8b9299"
    palette.base: "#101315"
    Item {
      id: image
      anchors.fill: parent
      Rectangle { anchors.fill: parent; color: "#101315" }
      Text {
        x: 12; y: 10; width: 454
        text: LANG === "es" ? "QML real · datos ficticios · vista aislada" : "Real QML · fictional data · isolated preview"
        color: "#9bbcff"; font.pixelSize: 12; font.family: "DejaVu Sans"
      }
      Plugin.ControlPanel { id: panel; x: 24; y: 40; backend: fixture }
    }
    FIXTURE
    property var shots: SHOTS
    property int shot: 0
    function findItems(item, predicate, result) {
      if (predicate(item)) result.push(item)
      if (item.children) for (var i = 0; i < item.children.length; i++) findItems(item.children[i], predicate, result)
      return result
    }
    function nextShot() {
      if (shot === shots.length) { Qt.quit(); return }
      var scene = shots[shot]
      fixture.available = false
      fixture.managedAgentRunning = false
      fixture.role = ""
      fixture.phase = ""
      fixture.phaseText = LANG === "es" ? "sin sesión" : "no active session"
      fixture.pairSas = ""
      fixture.installed = scene.name !== "setup"
      fixture.firewallPreview = ""
      fixture.firewallPort = ""
      fixture.receivingFile = false
      fixture.fileListening = false
      fixture.copiedFilePath = ""
      fixture.fileSessionActive = scene.tab === "files" || scene.state === "ready" || scene.state === "receiver"
      if (scene.state === "pair") fixture.pairSas = "123456"
      if (scene.state === "ready" || scene.state === "receiver" || scene.tab === "files") {
        fixture.available = true
        fixture.managedAgentRunning = true
        fixture.role = scene.state === "ready" ? "connect" : "serve"
        fixture.phase = scene.state === "ready" ? "ready" : scene.tab === "files" ? "connected" : "controlling"
        fixture.phaseText = scene.state === "ready" ? (LANG === "es" ? "listo" : "ready") : scene.tab === "files" ? (LANG === "es" ? "conectado" : "connected") : (LANG === "es" ? "control remoto" : "controlling")
      }
      if (scene.state === "firewall") {
        fixture.firewallPort = "47832"
        fixture.firewallPreview = "ufw allow in on demo0 from 192.0.2.0/24 to 192.0.2.20 port 47832 proto tcp comment seamlesscontrol"
      }
      if (scene.tab === "files") {
        fixture.receivingFile = true
        fixture.fileListening = true
      }
      panel.switchTab(scene.tab)
      var fields = findItems(panel, function(item) { return item.placeholderText !== undefined }, [])
      for (var i = 0; i < fields.length; i++) {
        var placeholder = String(fields[i].placeholderText)
        if (placeholder.indexOf("Destination folder") >= 0 || placeholder.indexOf("Directorio de destino") >= 0) fields[i].text = "/home/demo/Downloads"
        if (placeholder.indexOf("Absolute file") >= 0 || placeholder.indexOf("Ruta absoluta") >= 0) fields[i].text = "/home/demo/Documents/example.txt"
        if (placeholder.indexOf("Receiver IP:47833") >= 0 || placeholder.indexOf("IP del destino:47833") >= 0) fields[i].text = "192.0.2.20:47833"
      }
      settle.restart()
    }
    Timer {
      id: settle; interval: 350
      onTriggered: {
        var scene = window.shots[window.shot]
        var scrolls = window.findItems(panel, function(item) { return item.contentY !== undefined && item.contentHeight !== undefined }, [])
        if (scrolls.length) scrolls[0].contentY = scene.bottom ? Math.max(0, scrolls[0].contentHeight - scrolls[0].height) : 0
        window.height = Math.min(1560, panel.children[panel.children.length - 1].height + 64)
        capture.restart()
      }
    }
    Timer {
      id: capture; interval: 350
      onTriggered: image.grabToImage(function(result) {
        var path = OUT + "/" + window.shots[window.shot].name + "-" + LANG + ".png"
        if (!result.saveToFile(path)) { console.error("SAVE FAILED " + path); Qt.quit(); return }
        console.log("SAVED " + path)
        window.shot++
        window.nextShot()
      }, Qt.size(Math.round(image.width * 1.5), Math.round(image.height * 1.5)))
    }
    Component.onCompleted: nextShot()
  }
}
'''


def fixture_qml(language):
    source = (ROOT / "plugin/AgentBackend.qml").read_text()
    defaults = {"string": '""', "bool": "false", "int": "0", "var": "null"}
    properties = re.findall(r"^  (?:readonly )?property (string|bool|int|var) (\w+):", source, re.M)
    values = {
        "language": language, "installed": True, "pluginVersion": "0.24.4", "agentVersion": "0.24.4",
        "peers": [{"ip": "192.0.2.20", "key": "abcdef0123456789" * 4}],
        "peerPolicies": {"abcdef0123456789" * 4: {"control": True, "text": True, "files": True, "lastConnectedMs": 0}},
        "edgePolicy": "fluid",
        "discovered": [{"ip": "192.0.2.20", "key": "abcdef0123456789" * 4, "name": "Demo receiver", "address": "192.0.2.20:47832"}],
        "topology": [{"id": "local", "column": 0, "row": 0}, {"id": "192.0.2.20", "column": 1, "row": 0}],
        "peer": "192.0.2.20", "fileLimitMiB": 100, "approvalMode": "always", "approvalMinutes": 15,
        "approvalUntil": {}, "clipboardFileListening": True, "fileListenEndpoint": "192.0.2.20:47833",
    }
    declarations = [f"property {kind} {name}: {json.dumps(values[name]) if name in values else defaults[kind]}" for kind, name in properties]
    return "QtObject { id: fixture\n" + "\n".join(declarations) + "\nsignal pathChosen(string kind, string path)\nfunction cancelFirewall() {}\nfunction canShareWith(address) { return fileSessionActive && String(address).indexOf(peer + ':') === 0 }\n}"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=ROOT / "docs/images")
    parser.add_argument("--shell-source", type=Path, default=Path("/usr/share/omarchy/shell"))
    args = parser.parse_args()
    binary = shutil.which("quickshell")
    if not binary:
        parser.error("quickshell is required; no synthetic screenshot fallback is used")
    args.output.mkdir(parents=True, exist_ok=True)
    scenes = [
        {"name": "omarchy-overview", "tab": "home"},
        {"name": "omarchy-peers", "tab": "computers", "state": "pair"},
        {"name": "omarchy-map", "tab": "computers", "bottom": True},
        {"name": "session", "tab": "home", "state": "ready"},
        {"name": "omarchy-session-receiver", "tab": "home", "state": "receiver"},
        {"name": "firewall", "tab": "settings", "state": "firewall"},
        {"name": "omarchy-approval", "tab": "settings"},
        {"name": "omarchy-files-receive", "tab": "files"},
        {"name": "omarchy-files-send", "tab": "files", "bottom": True},
    ]
    with tempfile.TemporaryDirectory(prefix="seamlesscontrol-doc-qml-") as temporary:
        work = Path(temporary)
        for directory in ("Ui", "Commons"):
            shutil.copytree(args.shell_source / directory, work / directory)
        (work / "Ui/KeyboardPanel.qml").write_text(SURFACE)
        (work / "plugin").mkdir()
        for name in ("Panel.qml", "StateButton.qml", "HelpDisclosure.qml", "SafeTextField.qml", "Translations.js"):
            destination = "ControlPanel.qml" if name == "Panel.qml" else name
            shutil.copyfile(ROOT / "plugin" / name, work / "plugin" / destination)
        home = work / "home"
        home.mkdir()
        runtime = work / "runtime"
        runtime.mkdir(mode=0o700)
        env = os.environ.copy()
        for key in ("HYPRLAND_INSTANCE_SIGNATURE", "WAYLAND_DISPLAY", "DISPLAY", "DBUS_SESSION_BUS_ADDRESS"):
            env.pop(key, None)
        env.update(HOME=str(home), XDG_CONFIG_HOME=str(home / ".config"), XDG_CACHE_HOME=str(home / ".cache"),
                   XDG_DATA_HOME=str(home / ".local/share"), XDG_STATE_HOME=str(home / ".local/state"),
                   XDG_RUNTIME_DIR=str(runtime), QT_QPA_PLATFORM="offscreen", QT_QUICK_BACKEND="software",
                   QT_QPA_PLATFORMTHEME="", QT_STYLE_OVERRIDE="Fusion", QT_QUICK_CONTROLS_STYLE="Basic")
        for language in ("en", "es"):
            qml = SHELL.replace("FIXTURE", fixture_qml(language)).replace("SHOTS", json.dumps(scenes))
            qml = qml.replace("LANG", json.dumps(language)).replace("OUT", json.dumps(str(args.output.resolve())))
            (work / "shell.qml").write_text(qml)
            result = subprocess.run([binary, "--no-color", "--path", str(work / "shell.qml")], env=env,
                                    text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=60)
            print(result.stdout, end="")
            if result.returncode or result.stdout.count("SAVED ") != len(scenes):
                raise SystemExit("QML rendering failed; do not use incomplete generated media")


if __name__ == "__main__":
    main()
