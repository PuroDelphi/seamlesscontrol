#!/usr/bin/env bash
set -euo pipefail

runner=${QMLTESTRUNNER:-/usr/lib/qt6/bin/qmltestrunner}
if [[ ! -x $runner ]]; then
  printf 'qmltestrunner is required for the local QML keyboard check\n' >&2
  exit 2
fi
repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
scratch=$(mktemp -d)
trap 'rm -rf -- "$scratch"' EXIT
# qmltestrunner cannot load Quickshell.Io outside a ShellRoot. Stub only its
# Process sink; the real SafeTextField source supplies the keyboard logic.
sed '/^import Quickshell.Io$/d' "$repo_dir/plugin/SafeTextField.qml" > "$scratch/SafeTextField.qml"
cat > "$scratch/Process.qml" <<'QML'
import QtQuick
Item { property var command: []; property bool running: false; property var stdout: null }
QML
cat > "$scratch/StdioCollector.qml" <<'QML'
import QtQuick
Item { property bool waitForEnd: false; property string text: ""; signal streamFinished() }
QML
cat > "$scratch/tst_safe_field.qml" <<'QML'
import QtQuick
import QtTest
Item {
  width: 400
  height: 100
  SafeTextField { id: field; width: 300; height: 40 }
  TestCase {
    name: "SafeTextField"
    when: windowShown
    function test_keyboard_editing() {
      verify(field.readOnly)
      field.forceActiveFocus()
      keyClick(Qt.Key_A)
      keyClick(Qt.Key_B)
      compare(field.text, "ab")
      keyClick(Qt.Key_Backspace)
      compare(field.text, "a")
      keyClick(Qt.Key_A, Qt.ControlModifier)
      keyClick(Qt.Key_C)
      compare(field.text, "c")
    }
  }
}
QML
env -u DISPLAY -u WAYLAND_DISPLAY -u DBUS_SESSION_BUS_ADDRESS \
  QT_QPA_PLATFORM=offscreen QT_QPA_PLATFORMTHEME='' QT_STYLE_OVERRIDE=Fusion \
  QT_QUICK_CONTROLS_STYLE=Basic QT_QUICK_BACKEND=software \
  "$runner" -input "$scratch/tst_safe_field.qml" -o -,txt
