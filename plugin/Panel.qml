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
  readonly property var ownerItem: hostWidget || root
  readonly property color ink: bar ? bar.foreground : Color.foreground
  readonly property color muted: Qt.darker(ink, 1.5)
  readonly property string face: bar ? bar.fontFamily : Style.font.family

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
          text: "DOS EQUIPOS · UNA ENTRADA"
          foreground: root.ink
          fontFamily: root.face
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
          text: "1  Compile el agente en ambos equipos.\n2  En el destino: seamlesscontrold serve <IP:PUERTO>\n3  En el origen: seamlesscontrold connect <IP:PUERTO> right\n4  Compare el código de seis cifras en ambos paneles."
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
          text: "Escape devuelve el control local. La prueba física entre dos Omarchy y la configuración visual de pares siguen pendientes."
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
