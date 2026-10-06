import QtQuick
import QtQuick.Layouts
import qs.Commons
import qs.Ui

ColumnLayout {
  id: root

  property string title: ""
  property string description: ""
  property bool expanded: false
  property color foreground: Color.foreground
  property color detailColor: Qt.darker(foreground, 1.5)
  property string fontFamily: Style.font.family

  spacing: Style.space(6)

  StateButton {
    Layout.fillWidth: true
    text: (root.expanded ? "▾ " : "▸ ") + root.title
    bordered: true
    focusable: true
    leftAlign: true
    fontSize: Style.font.caption
    foreground: root.foreground
    accent: Color.accent
    fontFamily: root.fontFamily
    onClicked: root.expanded = !root.expanded
  }

  Text {
    Layout.fillWidth: true
    visible: root.expanded
    text: root.description
    textFormat: Text.PlainText
    wrapMode: Text.WordWrap
    color: root.detailColor
    font.family: root.fontFamily
    font.pixelSize: Style.font.caption
  }
}
