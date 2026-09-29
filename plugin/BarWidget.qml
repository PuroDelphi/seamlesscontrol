import QtQuick
import qs.Commons
import qs.Ui

BarWidget {
  id: root
  moduleName: "seamlesscontrol.control"

  implicitWidth: label.implicitWidth + Style.space(12)
  implicitHeight: barSize
  readonly property string stateLabel: !backend.available ? "sin agente"
    : backend.paused ? "pausa"
    : backend.phase === "controlling" ? "remoto"
    : backend.phase === "connected" ? "conectado"
    : backend.phase === "ready" ? "listo"
    : backend.phase === "listening" ? "disponible" : backend.phase

  function open() { if (panelLoader.item) panelLoader.item.open() }
  function close() { if (panelLoader.item) panelLoader.item.close() }
  function toggle() { if (panelLoader.item) panelLoader.item.toggle() }

  function attachPanel() {
    if (!panelLoader.item) return
    panelLoader.item.bar = root.bar
    panelLoader.item.anchorItem = root
    panelLoader.item.hostWidget = root
    panelLoader.item.backend = backend
  }

  onBarChanged: attachPanel()

  Text {
    id: label
    anchors.centerIn: parent
    text: "󰌘 " + root.stateLabel
    textFormat: Text.PlainText
    color: backend.available && !backend.paused ? Color.accent
      : root.bar ? root.bar.barForeground : Color.foreground
    font.family: root.bar ? root.bar.fontFamily : Style.font.family
    font.pixelSize: Style.font.body
  }

  MouseArea {
    anchors.fill: parent
    cursorShape: Qt.PointingHandCursor
    onClicked: root.toggle()
    onEntered: if (root.bar) root.bar.showTooltip(root, "SeamlessControl · " + root.stateLabel)
    onExited: if (root.bar) root.bar.hideTooltip(root)
  }

  AgentBackend { id: backend }

  Loader {
    id: panelLoader
    active: true
    source: Qt.resolvedUrl("Panel.qml")
    visible: false
    onLoaded: {
      root.attachPanel()
      Qt.callLater(root.attachPanel)
    }
  }
}
