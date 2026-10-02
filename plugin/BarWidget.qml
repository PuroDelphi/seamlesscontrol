import QtQuick
import qs.Commons
import qs.Ui
import "Translations.js" as Tr

BarWidget {
  id: root
  moduleName: "seamlesscontrol.control"

  implicitWidth: label.implicitWidth + Style.space(12)
  implicitHeight: barSize
  readonly property bool opened: panelLoader.item ? panelLoader.item.opened : false
  function t(spanish) { return Tr.text(spanish, backend.language) }
  readonly property string stateLabel: backend.fileOffer ? root.t("archivo pendiente")
    : backend.receivingFile ? root.t("esperando archivo")
    : backend.sendingFile ? root.t("enviando archivo")
    : !backend.installed ? root.t("sin agente")
    : !backend.available ? root.t("sin sesión")
    : backend.paused ? root.t("pausa")
    : backend.phase === "controlling" ? root.t("remoto")
    : backend.phase === "connected" ? root.t("conectado")
    : backend.phase === "ready" ? root.t("listo")
    : backend.phase === "rearming" ? root.t("alejar del borde")
    : backend.phase === "listening" ? root.t("disponible") : backend.phaseText

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
    color: backend.available && !backend.paused && backend.phase !== "locked" ? Color.accent
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
