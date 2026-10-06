import QtQuick
import qs.Ui as Ui

// Omarchy's Button keeps the same paint when disabled. Match the shell's
// dimmed controls without changing its theme colors, border, or sizing.
Ui.Button {
  opacity: enabled ? 1 : 0.4

  Behavior on opacity {
    NumberAnimation { duration: 120 }
  }
}
