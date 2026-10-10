import QtQuick
import QtQuick.Controls as Controls
import Quickshell.Io

// Qt 6.12's writable TextInput checks the clipboard on every selection change.
// Keep its native editor read-only so that check never reads a foreign Wayland
// offer in the shell. Keyboard edits are applied explicitly, and paste runs in
// a bounded helper process instead of through Qt's clipboard implementation.
Controls.TextField {
  id: root
  readOnly: true
  cursorVisible: activeFocus
  maximumLength: 4096

  signal edited()
  signal committed()
  signal safeEditingFinished()

  readonly property string pasteHelper: decodeURIComponent(
    String(Qt.resolvedUrl("../packaging/read-ui-clipboard.py")).replace(/^file:\/\//, ""))

  onActiveFocusChanged: if (!activeFocus) safeEditingFinished()

  function replaceSelection(value) {
    if (value.length === 0) return
    if (text.length - selectedText.length + value.length > maximumLength) return
    if (selectedText.length > 0) {
      var start = selectionStart
      remove(start, selectionEnd)
      cursorPosition = start
    }
    insert(cursorPosition, value)
    edited()
  }

  Keys.priority: Keys.BeforeItem
  Keys.onPressed: function(event) {
    if (!root.enabled) return
    var control = event.modifiers & Qt.ControlModifier
    var alt = event.modifiers & Qt.AltModifier
    var command = ((control || (event.modifiers & Qt.MetaModifier)) && !alt)
    var altGr = control && alt && !(event.modifiers & Qt.MetaModifier)
    if (command && event.key === Qt.Key_A) {
      root.selectAll()
    } else if (command && event.key === Qt.Key_V) {
      if (!pasteProcess.running) {
        pasteProcess.command = ["python3", root.pasteHelper]
        pasteProcess.running = true
      }
    } else if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
      root.committed()
    } else if (event.key === Qt.Key_Backspace || event.key === Qt.Key_Delete) {
      var start = root.selectionStart
      var end = root.selectionEnd
      if (root.selectedText.length === 0) {
        start = event.key === Qt.Key_Backspace ? Math.max(0, root.cursorPosition - 1) : root.cursorPosition
        end = event.key === Qt.Key_Delete ? Math.min(root.text.length, root.cursorPosition + 1) : root.cursorPosition
      }
      if (start !== end) {
        root.remove(start, end)
        root.cursorPosition = start
        root.edited()
      }
    } else if ((!command && !alt) || altGr) {
      if (event.text.length > 0 && event.text.charCodeAt(0) >= 32)
        root.replaceSelection(event.text)
      else {
        event.accepted = false
        return
      }
    } else {
      event.accepted = false
      return
    }
    event.accepted = true
  }

  Process {
    id: pasteProcess
    stdout: StdioCollector {
      waitForEnd: true
      onStreamFinished: {
        try {
          var result = JSON.parse(String(text || ""))
          if (typeof result.text === "string") root.replaceSelection(result.text)
        } catch (error) { /* An invalid or unavailable selection changes nothing. */ }
      }
    }
  }
}
