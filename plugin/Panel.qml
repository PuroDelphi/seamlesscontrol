import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as Controls
import Quickshell
import qs.Commons
import qs.Ui

Panel {
  id: root
  moduleName: "seamlesscontrol.control"
  manageIpc: false

  property var anchorItem: null
  property var hostWidget: null
  property var backend: null
  property string revokeCandidate: ""
  property string selectedMachine: ""
  readonly property var ownerItem: hostWidget || root
  readonly property color ink: bar ? bar.foreground : Color.foreground
  readonly property color muted: Qt.darker(ink, 1.5)
  readonly property string face: bar ? bar.fontFamily : Style.font.family
  readonly property var unassignedPeers: backend ? backend.peers.filter(function(peer) {
    return !backend.topology.some(function(slot) { return slot.id === peer.ip })
  }) : []

  function machineAt(column, row) {
    if (!backend) return ""
    for (var i = 0; i < backend.topology.length; i++) {
      var slot = backend.topology[i]
      if (slot.column === column && slot.row === row) {
        if (slot.id === "local" || backend.peers.some(function(peer) { return peer.ip === slot.id }))
          return slot.id
      }
    }
    return ""
  }

  function assignMachine(machine, column, row) {
    if (backend && machine) backend.placeMachine(machine, column, row)
    selectedMachine = ""
  }

  function open() { controller.show() }
  function close() { controller.hide() }
  function toggle() { opened ? close() : open() }

  KeyboardPanel {
    id: panel
    anchorItem: root.anchorItem
    owner: root.ownerItem
    bar: root.bar
    open: root.opened
    focusTarget: keyCatcher
    contentWidth: panel.fittedContentWidth(Style.space(430))
    contentHeight: panel.fittedContentHeight(content.implicitHeight)

    PanelKeyCatcher {
      id: keyCatcher
      anchors.fill: parent
      onCloseRequested: root.close()

      Flickable {
        id: scroller
        anchors.fill: parent
        contentWidth: width
        contentHeight: content.implicitHeight
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        flickableDirection: Flickable.VerticalFlick

        ColumnLayout {
          id: content
          width: scroller.width
          spacing: Style.space(12)

        PanelHero {
          Layout.fillWidth: true
          title: "SeamlessControl"
          meta: "OMARCHY  ·  " + (root.backend && root.backend.available
            ? root.backend.phaseText.toUpperCase() : "SIN AGENTE")
          foreground: root.ink
          fontFamily: root.face
          iconComponent: Component {
            Text {
              text: "󰌘"
              textFormat: Text.PlainText
              color: Color.accent
              font.family: root.face
              font.pixelSize: Style.font.display
            }
          }
        }

        PanelSeparator {
          Layout.fillWidth: true
          foreground: root.ink
        }

        PanelSectionHeader {
          Layout.fillWidth: true
          text: "HASTA CUATRO EQUIPOS · UNA ENTRADA"
          foreground: root.ink
          fontFamily: root.face
        }

        PanelSectionHeader {
          Layout.fillWidth: true
          text: "MAPA DE EQUIPOS · 2 × 2"
          foreground: root.ink
          fontFamily: root.face
        }

        Text {
          Layout.fillWidth: true
          text: "Arrastre un equipo a otra casilla o selecciónelo y pulse su destino. Los vecinos del equipo local definen el borde de salida."
          textFormat: Text.PlainText
          wrapMode: Text.WordWrap
          color: root.muted
          font.family: root.face
          font.pixelSize: Style.font.caption
        }

        GridLayout {
          id: machineGrid
          Layout.fillWidth: true
          columns: 2
          columnSpacing: Style.space(8)
          rowSpacing: Style.space(8)

          Repeater {
            model: 4
            delegate: Rectangle {
              id: gridCell
              required property int index
              readonly property int column: index % 2
              readonly property int row: Math.floor(index / 2)
              readonly property string machine: root.machineAt(column, row)
              Layout.fillWidth: true
              Layout.preferredHeight: Style.space(68)
              radius: 8
              color: Color.background
              border.width: 1
              border.color: root.selectedMachine !== "" ? Color.accent : Color.muted

              DropArea {
                anchors.fill: parent
                onDropped: function(drop) {
                  if (drop.source && drop.source.machineId)
                    root.assignMachine(drop.source.machineId, gridCell.column, gridCell.row)
                }
              }

              Rectangle {
                id: gridTile
                z: 1
                property string machineId: gridCell.machine
                visible: machineId !== ""
                width: gridCell.width - Style.space(8)
                height: gridCell.height - Style.space(8)
                x: Style.space(4)
                y: Style.space(4)
                radius: 6
                color: machineId === "local" ? Color.accent : Color.muted
                Drag.active: tileMouse.drag.active
                Drag.source: gridTile
                Drag.hotSpot.x: width / 2
                Drag.hotSpot.y: height / 2

                Text {
                  anchors.centerIn: parent
                  width: parent.width - Style.space(8)
                  text: gridTile.machineId === "local" ? "● ESTE EQUIPO" : "󰍹 " + gridTile.machineId
                  textFormat: Text.PlainText
                  horizontalAlignment: Text.AlignHCenter
                  elide: Text.ElideMiddle
                  color: Color.background
                  font.family: root.face
                  font.pixelSize: Style.font.caption
                }

                MouseArea {
                  id: tileMouse
                  anchors.fill: parent
                  drag.target: gridTile
                  onClicked: root.selectedMachine = gridTile.machineId
                  onReleased: {
                    Qt.callLater(function() {
                      gridTile.x = Style.space(4)
                      gridTile.y = Style.space(4)
                    })
                  }
                }
              }

              MouseArea {
                anchors.fill: parent
                onClicked: if (root.selectedMachine !== "")
                  root.assignMachine(root.selectedMachine, gridCell.column, gridCell.row)
              }
            }
          }
        }

        Text {
          Layout.fillWidth: true
          visible: root.unassignedPeers.length > 0
          text: "SIN POSICIÓN · " + root.unassignedPeers.map(function(peer) { return peer.ip }).join(" · ")
          textFormat: Text.PlainText
          wrapMode: Text.WordWrap
          color: Color.accent
          font.family: root.face
          font.pixelSize: Style.font.caption
        }

        Repeater {
          model: root.unassignedPeers.length
          delegate: Button {
            required property int index
            Layout.fillWidth: true
            text: "Ubicar " + root.unassignedPeers[index].ip
            bordered: true
            focusable: true
            foreground: root.ink
            accent: Color.accent
            fontFamily: root.face
            onClicked: root.selectedMachine = root.unassignedPeers[index].ip
          }
        }

        RowLayout {
          Layout.fillWidth: true
          visible: root.backend && !root.backend.available
          spacing: Style.space(8)
          Controls.TextField {
            id: pairAddress
            Layout.fillWidth: true
            placeholderText: "IP:puerto del destino"
            color: root.ink
            font.family: root.face
            background: Rectangle {
              color: "transparent"
              border.color: Color.accent
              border.width: 1
              radius: 8
            }
          }
          Button {
            text: root.backend && root.backend.pairingRunning ? "Conectando…" : "Emparejar"
            bordered: true
            focusable: true
            enabled: root.backend && !root.backend.pairingRunning && pairAddress.text !== ""
            foreground: root.ink
            accent: Color.accent
            fontFamily: root.face
            onClicked: if (root.backend) root.backend.pair(pairAddress.text.trim())
          }
        }

        PanelSectionHeader {
          Layout.fillWidth: true
          visible: root.backend && root.backend.peers.length > 0
          text: "EQUIPOS EMPAREJADOS"
          foreground: root.ink
          fontFamily: root.face
        }

        Repeater {
          model: root.backend ? root.backend.peers.length : 0
          delegate: RowLayout {
            id: peerRow
            required property int index
            Layout.fillWidth: true
            spacing: Style.space(8)
            readonly property var peer: root.backend.peers[peerRow.index]
            Text {
              Layout.fillWidth: true
              text: peerRow.peer.ip + " · " + peerRow.peer.key.slice(0, 12) + "…"
              textFormat: Text.PlainText
              color: root.ink
              elide: Text.ElideRight
              font.family: root.face
              font.pixelSize: Style.font.caption
            }
            Button {
              text: "Revocar"
              bordered: true
              focusable: true
              foreground: root.ink
              accent: Color.accent
              fontFamily: root.face
              onClicked: root.revokeCandidate = peerRow.peer.ip
            }
          }
        }

        RowLayout {
          Layout.fillWidth: true
          visible: root.revokeCandidate !== ""
          spacing: Style.space(8)
          Text {
            Layout.fillWidth: true
            text: "¿Revocar " + root.revokeCandidate + "? Esa clave no podrá volver a conectarse."
            textFormat: Text.PlainText
            wrapMode: Text.WordWrap
            color: Color.urgent
            font.family: root.face
            font.pixelSize: Style.font.caption
          }
          Button {
            text: "Confirmar"
            bordered: true
            focusable: true
            foreground: root.ink
            accent: Color.accent
            fontFamily: root.face
            onClicked: {
              if (root.backend) root.backend.revoke(root.revokeCandidate)
              root.revokeCandidate = ""
            }
          }
          Button {
            text: "Cancelar"
            bordered: true
            focusable: true
            foreground: root.ink
            accent: Color.accent
            fontFamily: root.face
            onClicked: root.revokeCandidate = ""
          }
        }

        Text {
          Layout.fillWidth: true
          visible: root.backend && root.backend.error !== ""
          text: root.backend ? root.backend.error : ""
          textFormat: Text.PlainText
          wrapMode: Text.WordWrap
          color: Color.urgent
          font.family: root.face
          font.pixelSize: Style.font.caption
        }

        Text {
          Layout.fillWidth: true
          text: root.backend && root.backend.available
            ? "Agente " + (root.backend.role === "serve" ? "receptor" : "emisor")
              + (root.backend.peer !== "" ? " · " + root.backend.peer : "")
              + (root.backend.paused ? " · en pausa" : "")
            : "El agente no está ejecutándose. Compile e inícielo en un terminal para comenzar."
          textFormat: Text.PlainText
          wrapMode: Text.WordWrap
          color: root.ink
          font.family: root.face
          font.pixelSize: Style.font.body
        }

        Text {
          Layout.fillWidth: true
          text: "1  Compile el agente en ambos equipos.\n2  En el destino: seamlesscontrold serve <IP:PUERTO>\n3  Empareje y ubique el destino en la cuadrícula.\n4  En el origen: seamlesscontrold connect <IP:PUERTO>\n5  Compare el código de seis cifras en ambos paneles."
          textFormat: Text.PlainText
          wrapMode: Text.WordWrap
          color: root.muted
          font.family: root.face
          font.pixelSize: Style.font.caption
        }

        PanelSectionHeader {
          Layout.fillWidth: true
          visible: root.backend && root.backend.pairSas !== ""
          text: "CONFIRMAR EMPAREJAMIENTO"
          foreground: root.ink
          fontFamily: root.face
        }

        Text {
          Layout.fillWidth: true
          visible: root.backend && root.backend.pairSas !== ""
          text: root.backend ? "Código: " + root.backend.pairSas + "\nClave remota: " + root.backend.pairKey : ""
          textFormat: Text.PlainText
          wrapMode: Text.WrapAnywhere
          color: Color.accent
          font.family: root.face
          font.pixelSize: Style.font.body
        }

        RowLayout {
          Layout.fillWidth: true
          visible: root.backend && root.backend.pairSas !== ""
          spacing: Style.space(8)
          Button {
            Layout.fillWidth: true
            text: "Coincide · aceptar"
            bordered: true
            focusable: true
            foreground: root.ink
            accent: Color.accent
            fontFamily: root.face
            onClicked: if (root.backend) root.backend.decidePair(true)
          }
          Button {
            Layout.fillWidth: true
            text: "Rechazar"
            bordered: true
            focusable: true
            foreground: root.ink
            accent: Color.accent
            fontFamily: root.face
            onClicked: if (root.backend) root.backend.decidePair(false)
          }
        }

        Button {
          Layout.fillWidth: true
          visible: root.backend && root.backend.available && root.backend.role === "connect"
          text: root.backend && root.backend.paused ? "Reanudar captura" : "Pausar captura"
          bordered: true
          focusable: true
          foreground: root.ink
          accent: Color.accent
          fontFamily: root.face
          onClicked: if (root.backend) root.backend.togglePause()
        }

        Text {
          Layout.fillWidth: true
          text: "Escape devuelve el control local. La prueba física entre dos Omarchy sigue pendiente. La cuadrícula aún no enruta entre varios pares conectados."
          textFormat: Text.PlainText
          wrapMode: Text.WordWrap
          color: Color.accent
          font.family: root.face
          font.pixelSize: Style.font.caption
        }

        Button {
          Layout.fillWidth: true
          text: "Abrir guía completa"
          bordered: true
          focusable: true
          foreground: root.ink
          accent: Color.accent
          fontFamily: root.face
          onClicked: {
            Quickshell.execDetached(["xdg-open", "https://github.com/PuroDelphi/seamlesscontrol#readme"])
            root.close()
          }
        }
        }
      }
    }
  }
}
