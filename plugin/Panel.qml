import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as Controls
import QtQuick.Dialogs as Dialogs
import QtCore as Core
import Quickshell
import qs.Commons
import qs.Ui
import "Translations.js" as Tr

Panel {
  id: root
  moduleName: "seamlesscontrol.control"
  manageIpc: false

  property var anchorItem: null
  property var hostWidget: null
  property var backend: null
  property string revokeCandidate: ""
  property string selectedMachine: ""
  property int keyboardCell: -1
  property bool confirmRemoveAgent: false
  readonly property var ownerItem: hostWidget || root
  readonly property color ink: bar ? bar.foreground : Color.foreground
  readonly property color muted: Qt.darker(ink, 1.5)
  readonly property string face: bar ? bar.fontFamily : Style.font.family
  readonly property var unassignedPeers: backend ? backend.peers.filter(function(peer) {
    return !backend.topology.some(function(slot) { return slot.id === peer.ip })
  }) : []
  readonly property int placedPeerCount: backend ? backend.topology.filter(function(slot) {
    return slot.id !== "local" && backend.peers.some(function(peer) { return peer.ip === slot.id })
  }).length : 0

  function t(spanish) { return Tr.text(spanish, backend ? backend.language : "en") }

  function formatFirewallPreview(rule) {
    return String(rule || "")
      .replace(/^(Proposed UFW rule: |Regla UFW propuesta: )/, "")
      .replace(" from ", "\nfrom ")
      .replace(" to ", "\nto ")
      .replace(" port ", "\nport ")
      .replace(" proto ", "\nproto ")
      .replace(" comment ", "\ncomment ")
  }

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

  function focusCell(index) {
    keyboardCell = index
    var cell = machineCells.itemAt(index)
    if (cell) cell.forceActiveFocus()
  }

  function moveKeyboardCell(dx, dy) {
    if (keyboardCell < 0) return
    var column = keyboardCell % 2 + dx
    var row = Math.floor(keyboardCell / 2) + dy
    if (column >= 0 && column < 2 && row >= 0 && row < 2)
      focusCell(row * 2 + column)
  }

  function activateKeyboardCell() {
    if (keyboardCell < 0) return
    var column = keyboardCell % 2
    var row = Math.floor(keyboardCell / 2)
    if (selectedMachine !== "") assignMachine(selectedMachine, column, row)
    else {
      var machine = machineAt(column, row)
      if (machine !== "") selectedMachine = machine
    }
  }

  function knownServer(server) {
    return backend && backend.peers.some(function(peer) {
      return peer.ip === server.ip && peer.key === server.key
    })
  }

  function changedServerKey(server) {
    return backend && backend.peers.some(function(peer) {
      return peer.ip === server.ip && peer.key !== server.key
    })
  }

  function previousServerIp(server) {
    if (!backend) return ""
    var previous = backend.peers.find(function(peer) {
      return peer.ip !== server.ip && peer.key === server.key
    })
    return previous ? previous.ip : ""
  }

  function adjacentServer(server) {
    if (!backend) return false
    var local = backend.topology.find(function(slot) { return slot.id === "local" })
    var remote = backend.topology.find(function(slot) { return slot.id === server.ip })
    return local && remote
      && Math.abs(local.column - remote.column) + Math.abs(local.row - remote.row) === 1
  }

  function captureEdge() {
    if (!backend || !backend.peer) return ""
    var local = backend.topology.find(function(slot) { return slot.id === "local" })
    var remote = backend.topology.find(function(slot) { return slot.id === backend.peer })
    if (!local || !remote) return ""
    if (remote.column === local.column - 1 && remote.row === local.row) return root.t("izquierdo")
    if (remote.column === local.column + 1 && remote.row === local.row) return root.t("derecho")
    if (remote.row === local.row - 1 && remote.column === local.column) return root.t("superior")
    if (remote.row === local.row + 1 && remote.column === local.column) return root.t("inferior")
    return ""
  }

  function open() { controller.show() }
  function close() {
    keyboardCell = -1
    selectedMachine = ""
    controller.hide()
  }
  function toggle() { opened ? close() : open() }

  function localPath(url) {
    return decodeURIComponent(String(url).replace(/^file:\/\/(localhost)?/, ""))
  }

  Dialogs.FileDialog {
    id: sourceChooser
    title: root.t("Archivo para SeamlessControl")
    fileMode: Dialogs.FileDialog.OpenFile
    onAccepted: fileSourcePath.text = root.localPath(selectedFile)
  }

  Dialogs.FolderDialog {
    id: destinationChooser
    title: root.t("Guardar archivos de SeamlessControl")
    onAccepted: fileDirectory.text = root.localPath(selectedFolder)
  }

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
      onTabRequested: function(direction) {
        root.focusCell(root.keyboardCell < 0 ? (direction > 0 ? 0 : 3) : (root.keyboardCell + direction + 4) % 4)
      }
      onMoveRequested: function(dx, dy) { root.moveKeyboardCell(dx, dy) }
      onActivateRequested: root.activateKeyboardCell()
      onCloseRequested: {
        if (root.selectedMachine !== "") root.selectedMachine = ""
        else root.close()
      }

      Flickable {
        id: scroller
        anchors.fill: parent
        contentWidth: width
        contentHeight: content.implicitHeight
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        flickableDirection: Flickable.VerticalFlick

        Connections {
          target: root.backend
          function onPairSasChanged() {
            if (root.backend && root.backend.pairSas !== "") {
              scroller.contentY = 0
              root.open()
            }
          }
        }

        ColumnLayout {
          id: content
          width: scroller.width
          spacing: Style.space(12)

        PanelHero {
          Layout.fillWidth: true
          title: "SeamlessControl"
          meta: "OMARCHY  ·  " + (root.backend && !root.backend.installed ? root.t("SIN AGENTE")
            : root.backend && root.backend.available ? root.backend.phaseText.toUpperCase() : root.t("SIN SESIÓN"))
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

        RowLayout {
          Layout.fillWidth: true
          spacing: Style.space(8)
          Text {
            Layout.fillWidth: true
            text: root.t("IDIOMA")
            color: root.muted
            font.family: root.face
            font.pixelSize: Style.font.caption
          }
          Button {
            text: "English"
            bordered: true
            focusable: true
            enabled: root.backend && root.backend.language !== "en"
            foreground: root.ink
            accent: Color.accent
            fontFamily: root.face
            onClicked: if (root.backend) root.backend.setLanguage("en")
          }
          Button {
            text: "Español"
            bordered: true
            focusable: true
            enabled: root.backend && root.backend.language !== "es"
            foreground: root.ink
            accent: Color.accent
            fontFamily: root.face
            onClicked: if (root.backend) root.backend.setLanguage("es")
          }
        }

        PanelSectionHeader {
          Layout.fillWidth: true
          text: root.t("PREPARAR ESTE EQUIPO")
          foreground: root.ink
          fontFamily: root.face
        }

        Text {
          Layout.fillWidth: true
          text: root.backend && root.backend.installed
            ? root.t("El agente ya está instalado. Detenga las sesiones activas antes de actualizarlo. Se abrirá una terminal para mostrar el progreso.")
            : root.t("Después de añadir el plugin con Omarchy, instale aquí el agente y los paquetes que falten. Se abrirá una terminal para mostrar el progreso y pedir autorización si hace falta.")
          textFormat: Text.PlainText
          wrapMode: Text.WordWrap
          color: root.muted
          font.family: root.face
          font.pixelSize: Style.font.caption
        }

        RowLayout {
          Layout.fillWidth: true
          spacing: Style.space(8)
          Button {
            Layout.fillWidth: true
            text: root.backend && root.backend.installed
              ? root.t("Actualizar agente") : root.t("Instalar agente")
            bordered: true
            focusable: true
            enabled: root.backend && !root.backend.available && !root.backend.managedAgentRunning
              && !root.backend.setupBusy
            foreground: root.ink
            accent: Color.accent
            fontFamily: root.face
            onClicked: if (root.backend) root.backend.runSetup("install")
          }
          Button {
            Layout.fillWidth: true
            text: root.t("Retirar agente")
            bordered: true
            focusable: true
            enabled: root.backend && root.backend.installed && !root.backend.available
              && !root.backend.setupBusy && !root.backend.managedAgentRunning
            foreground: root.ink
            accent: Color.accent
            fontFamily: root.face
            onClicked: root.confirmRemoveAgent = true
          }
        }

        Text {
          Layout.fillWidth: true
          visible: root.confirmRemoveAgent
          text: root.t("Se retirará el agente y solo los paquetes que instaló SeamlessControl. Las claves y equipos emparejados se conservarán. Después puede quitar el widget con omarchy plugin remove seamlesscontrol.control.")
          textFormat: Text.PlainText
          wrapMode: Text.WordWrap
          color: Color.urgent
          font.family: root.face
          font.pixelSize: Style.font.caption
        }

        RowLayout {
          Layout.fillWidth: true
          visible: root.confirmRemoveAgent
          spacing: Style.space(8)
          Button {
            Layout.fillWidth: true
            text: root.t("Confirmar retirada")
            bordered: true
            focusable: true
            foreground: root.ink
            accent: Color.urgent
            fontFamily: root.face
            onClicked: {
              root.confirmRemoveAgent = false
              if (root.backend) root.backend.runSetup("remove")
            }
          }
          Button {
            Layout.fillWidth: true
            text: root.t("Cancelar")
            bordered: true
            focusable: true
            foreground: root.ink
            accent: Color.accent
            fontFamily: root.face
            onClicked: root.confirmRemoveAgent = false
          }
        }

        Text {
          Layout.fillWidth: true
          visible: root.backend && (root.backend.setupMessage !== "" || root.backend.setupError !== "")
          text: root.backend ? root.t(root.backend.setupError !== "" ? root.backend.setupError : root.backend.setupMessage) : ""
          textFormat: Text.PlainText
          wrapMode: Text.WordWrap
          color: root.backend && root.backend.setupError !== "" ? Color.urgent : Color.accent
          font.family: root.face
          font.pixelSize: Style.font.caption
        }

        PanelSeparator {
          Layout.fillWidth: true
          foreground: root.ink
        }

        PanelSectionHeader {
          Layout.fillWidth: true
          visible: root.backend && root.backend.pairSas !== ""
          text: root.t("CONFIRMAR EMPAREJAMIENTO · ESTE EQUIPO")
          foreground: root.ink
          fontFamily: root.face
        }

        Text {
          Layout.fillWidth: true
          visible: root.backend && root.backend.pairSas !== ""
          text: root.backend ? root.t("Código de este equipo: ") + root.backend.pairSas
            + root.t("\nCompárelo con el que aparece en el otro Omarchy. Se genera automáticamente; no hay que escribirlo ni cambiarlo.")
            : ""
          textFormat: Text.PlainText
          wrapMode: Text.WordWrap
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
            text: root.t("Coincide · aprobar aquí")
            bordered: true
            focusable: true
            foreground: root.ink
            accent: Color.accent
            fontFamily: root.face
            onClicked: if (root.backend) root.backend.decidePair(true)
          }
          Button {
            Layout.fillWidth: true
            text: root.t("No coincide · rechazar")
            bordered: true
            focusable: true
            foreground: root.ink
            accent: Color.accent
            fontFamily: root.face
            onClicked: if (root.backend) root.backend.decidePair(false)
          }
        }

        PanelSectionHeader {
          Layout.fillWidth: true
          text: root.t("HASTA CUATRO EQUIPOS · UNA ENTRADA")
          foreground: root.ink
          fontFamily: root.face
        }

        PanelSectionHeader {
          Layout.fillWidth: true
          text: root.t("MAPA DE EQUIPOS · 2 × 2")
          foreground: root.ink
          fontFamily: root.face
        }

        Text {
          Layout.fillWidth: true
          text: root.t("Ubique el otro equipo junto a ESTE EQUIPO en ambos Omarchy. En el receptor, esa posición permite volver cruzando el borde hacia el origen; sin ella, use Escape o «Devolver control al origen».")
          textFormat: Text.PlainText
          wrapMode: Text.WordWrap
          color: root.muted
          font.family: root.face
          font.pixelSize: Style.font.caption
        }

        Text {
          Layout.fillWidth: true
          text: root.t("El mapa guarda la dirección del cruce. Para iniciar la sesión, pulse Conectar en el origen.")
          textFormat: Text.PlainText
          wrapMode: Text.WordWrap
          color: Color.accent
          font.family: root.face
          font.pixelSize: Style.font.caption
        }

        Text {
          Layout.fillWidth: true
          text: root.t("Teclado: Tab recorre las casillas, Enter elige una ficha, las flechas llevan a la casilla de destino y Enter la coloca. Escape cancela la selección.")
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
            id: machineCells
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
              border.width: root.keyboardCell === gridCell.index ? 2 : 1
              border.color: root.keyboardCell === gridCell.index || root.selectedMachine !== "" ? Color.accent : Color.muted

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
                  text: gridTile.machineId === "local" ? root.t("● ESTE EQUIPO") : "󰍹 " + gridTile.machineId
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
                  onClicked: {
                    root.selectedMachine = gridTile.machineId
                    gridCell.forceActiveFocus()
                  }
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
                onClicked: {
                  gridCell.forceActiveFocus()
                  if (root.selectedMachine !== "")
                    root.assignMachine(root.selectedMachine, gridCell.column, gridCell.row)
                }
              }
            }
          }
        }

        Text {
          Layout.fillWidth: true
          visible: root.unassignedPeers.length > 0
          text: root.t("SIN POSICIÓN · ") + root.unassignedPeers.map(function(peer) { return peer.ip }).join(" · ")
            + root.t("\nUbique este equipo en la cuadrícula antes de probar el regreso por el borde.")
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
            text: root.t("Ubicar ") + root.unassignedPeers[index].ip
            bordered: true
            focusable: true
            foreground: root.ink
            accent: Color.accent
            fontFamily: root.face
            onClicked: {
              root.selectedMachine = root.unassignedPeers[index].ip
              root.focusCell(0)
            }
          }
        }

        PanelSectionHeader {
          Layout.fillWidth: true
          text: root.t("EQUIPOS EN LA RED")
          foreground: root.ink
          fontFamily: root.face
        }

        Text {
          Layout.fillWidth: true
          text: root.t("Emparejar autoriza un equipo una sola vez. Conectar inicia cada sesión de control.")
          textFormat: Text.PlainText
          wrapMode: Text.WordWrap
          color: Color.accent
          font.family: root.face
          font.pixelSize: Style.font.caption
        }

        RowLayout {
          Layout.fillWidth: true
          spacing: Style.space(8)
          Text {
            Layout.fillWidth: true
            text: root.backend && root.backend.discoveryError !== ""
              ? root.backend.discoveryError
              : root.backend && root.backend.discovered.length > 0
                ? root.t("Compare el código al emparejar. Una IP nueva se verifica con la clave guardada.")
                : root.t("Sin receptores encontrados. Abra Recibir control en el otro Omarchy.")
            textFormat: Text.PlainText
            wrapMode: Text.WordWrap
            color: root.muted
            font.family: root.face
            font.pixelSize: Style.font.caption
          }
          Button {
            text: root.t("Buscar")
            bordered: true
            focusable: true
            enabled: root.backend && root.backend.installed
            foreground: root.ink
            accent: Color.accent
            fontFamily: root.face
            onClicked: if (root.backend) root.backend.refreshDiscovery()
          }
        }

        Repeater {
          model: root.backend ? root.backend.discovered.length : 0
          delegate: RowLayout {
            id: discoveredRow
            required property int index
            readonly property var server: root.backend.discovered[discoveredRow.index]
            Layout.fillWidth: true
            spacing: Style.space(8)
            Text {
              Layout.fillWidth: true
              text: discoveredRow.server.name
              textFormat: Text.PlainText
              elide: Text.ElideRight
              color: root.ink
              font.family: root.face
              font.pixelSize: Style.font.caption
            }
            Button {
              Layout.preferredWidth: Style.space(108)
              text: root.changedServerKey(discoveredRow.server) ? root.t("Clave cambió")
                : root.knownServer(discoveredRow.server)
                  ? root.adjacentServer(discoveredRow.server) ? root.t("Conectar") : root.t("Ubicar")
                  : root.previousServerIp(discoveredRow.server) !== "" ? "Actualizar IP" : root.t("Emparejar")
              bordered: true
              focusable: true
              enabled: root.backend && root.backend.installed && !root.backend.available
                && !root.backend.pairingRunning && !root.backend.managedAgentRunning
                && !root.changedServerKey(discoveredRow.server)
              foreground: root.ink
              accent: Color.accent
              fontFamily: root.face
              onClicked: {
                if (!root.backend) return
                if (root.knownServer(discoveredRow.server)) {
                  if (root.adjacentServer(discoveredRow.server))
                    root.backend.startSender(discoveredRow.server.address)
                  else root.selectedMachine = discoveredRow.server.ip
                } else root.backend.pair(discoveredRow.server.address)
              }
            }
          }
        }

        Text {
          Layout.fillWidth: true
          visible: root.backend && root.backend.available && root.backend.role === "connect"
          text: root.backend && root.backend.phase === "connecting" && root.backend.peer !== ""
            ? root.t("Red conectada. Preparando la captura del ratón y teclado…")
            : root.backend && root.backend.phase === "connecting"
              ? root.t("Conectando con el receptor. Compruebe que allí sigue activo Recibir control.")
            : root.backend && root.backend.phase === "reconnecting"
              ? root.t("Buscando el receptor de nuevo. Compruebe que allí sigue activo Recibir control y que el puerto está permitido en su firewall.")
              : root.backend && (root.backend.phase === "ready" || root.backend.phase === "rearming")
                ? (root.captureEdge() !== ""
                  ? root.t("Listo. Cruce el borde exterior ") + root.captureEdge() + root.t(" de las pantallas de este equipo. Si acaba de volver, aleje primero el puntero del borde.")
                  : root.t("Listo. Revise la posición del otro equipo en el mapa para saber por qué borde cruzar."))
                : ""
          textFormat: Text.PlainText
          wrapMode: Text.WordWrap
          color: Color.accent
          font.family: root.face
          font.pixelSize: Style.font.caption
        }

        Text {
          Layout.fillWidth: true
          visible: root.backend && root.backend.captureWaitSeconds >= 15
            && root.backend.managedAgentRunning
          text: root.t("La preparación de la captura tarda demasiado. Reiniciar captura cerrará esta sesión y puede interrumpir otras aplicaciones que comparten pantalla en este equipo. Después pulse Conectar otra vez.")
          textFormat: Text.PlainText
          wrapMode: Text.WordWrap
          color: Color.urgent
          font.family: root.face
          font.pixelSize: Style.font.caption
        }

        Button {
          Layout.fillWidth: true
          visible: root.backend && root.backend.captureWaitSeconds >= 15
            && root.backend.managedAgentRunning
          text: root.t("Reiniciar captura de este equipo")
          bordered: true
          focusable: true
          enabled: root.backend && !root.backend.repairBusy
          foreground: root.ink
          accent: Color.urgent
          fontFamily: root.face
          onClicked: if (root.backend) root.backend.repairCapture()
        }

        Text {
          Layout.fillWidth: true
          visible: root.backend && (root.backend.repairBusy || root.backend.repairMessage !== "" || root.backend.repairError !== "")
          text: root.backend && root.backend.repairBusy ? root.t("Reiniciando la captura…")
            : root.backend && root.backend.repairError !== "" ? root.backend.repairError
            : root.backend ? root.backend.repairMessage : ""
          textFormat: Text.PlainText
          wrapMode: Text.WordWrap
          color: root.backend && root.backend.repairError !== "" ? Color.urgent : Color.accent
          font.family: root.face
          font.pixelSize: Style.font.caption
        }

        RowLayout {
          Layout.fillWidth: true
          visible: root.backend && !root.backend.available
          spacing: Style.space(8)
          Controls.TextField {
            id: pairAddress
            Layout.fillWidth: true
            placeholderText: root.t("Alternativa manual · IP:puerto")
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
            text: root.backend && root.backend.pairingRunning ? root.t("Conectando…") : root.t("Emparejar")
            bordered: true
            focusable: true
            enabled: root.backend && root.backend.installed && !root.backend.pairingRunning && pairAddress.text !== ""
            foreground: root.ink
            accent: Color.accent
            fontFamily: root.face
            onClicked: if (root.backend) root.backend.pair(pairAddress.text.trim())
          }
        }

        PanelSectionHeader {
          Layout.fillWidth: true
          visible: root.backend && root.backend.peers.length > 0
          text: root.t("EQUIPOS EMPAREJADOS")
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
              text: root.t("Revocar")
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
            text: root.t("¿Revocar ") + root.revokeCandidate + root.t("? Esa clave no podrá volver a conectarse.")
            textFormat: Text.PlainText
            wrapMode: Text.WordWrap
            color: Color.urgent
            font.family: root.face
            font.pixelSize: Style.font.caption
          }
          Button {
            text: root.t("Confirmar")
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
            text: root.t("Cancelar")
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
            ? root.t("Agente ") + (root.backend.role === "serve" ? root.t("receptor") : root.t("emisor"))
              + (root.backend.peer !== "" ? " · " + root.backend.peer : "")
              + (root.backend.paused ? root.t(" · en pausa") : "")
            : root.backend && !root.backend.installed
              ? root.t("Falta el agente. Pulse Instalar agente arriba; el panel lo detectará automáticamente.")
              : root.t("No hay sesión activa. Iníciela desde este panel o desde un terminal.")
          textFormat: Text.PlainText
          wrapMode: Text.WordWrap
          color: root.ink
          font.family: root.face
          font.pixelSize: Style.font.body
        }

        PanelSectionHeader {
          Layout.fillWidth: true
          text: root.t("INICIAR SESIÓN")
          foreground: root.ink
          fontFamily: root.face
        }

        RowLayout {
          Layout.fillWidth: true
          visible: root.backend && !root.backend.available && !root.backend.managedAgentRunning
          spacing: Style.space(8)
          Controls.TextField {
            id: listenAddress
            Layout.fillWidth: true
            placeholderText: root.t("Automático · 47832 (o puerto / IP:puerto)")
            color: root.ink
            font.family: root.face
            background: Rectangle { color: "transparent"; border.color: Color.accent; border.width: 1; radius: 8 }
            onTextChanged: {
              var value = text.trim()
              var match = value.match(/:([0-9]{1,5})$/)
              if (match) firewallPortField.text = match[1]
              else if (/^[0-9]{1,5}$/.test(value)) firewallPortField.text = value
            }
          }
          Button {
            text: root.t("Recibir control")
            bordered: true
            focusable: true
            enabled: root.backend && root.backend.installed && !root.backend.pairingRunning
            foreground: root.ink
            accent: Color.accent
            fontFamily: root.face
            onClicked: if (root.backend) {
              var address = listenAddress.text.trim()
              if (address === "") root.backend.startReceiverAuto(firewallPortField.text.trim())
              else if (/^[0-9]{1,5}$/.test(address)) root.backend.startReceiverAuto(address)
              else root.backend.startReceiver(address)
            }
          }
        }

        PanelSectionHeader {
          Layout.fillWidth: true
          visible: root.backend && root.backend.installed
          text: root.t("FIREWALL · SOLO EN EL RECEPTOR")
          foreground: root.ink
          fontFamily: root.face
        }

        Text {
          Layout.fillWidth: true
          visible: root.backend && root.backend.installed
          text: root.t("Si el otro equipo agota el tiempo de conexión, permita aquí el mismo puerto TCP elegido para «Recibir control». Primero verá la regla limitada a esta red local; Omarchy pedirá autorización antes de aplicarla.")
          textFormat: Text.PlainText
          wrapMode: Text.WordWrap
          color: root.muted
          font.family: root.face
          font.pixelSize: Style.font.caption
        }

        RowLayout {
          Layout.fillWidth: true
          visible: root.backend && root.backend.installed
          spacing: Style.space(8)
          Controls.TextField {
            id: firewallPortField
            Layout.fillWidth: true
            text: "47832"
            placeholderText: root.t("Puerto TCP del receptor")
            color: root.ink
            font.family: root.face
            background: Rectangle { color: "transparent"; border.color: Color.accent; border.width: 1; radius: 8 }
            onTextChanged: if (root.backend) root.backend.cancelFirewall()
          }
          Button {
            text: root.backend && root.backend.firewallBusy ? root.t("Comprobando…") : root.t("Preparar regla LAN")
            bordered: true
            focusable: true
            enabled: root.backend && !root.backend.firewallBusy
            foreground: root.ink
            accent: Color.accent
            fontFamily: root.face
            onClicked: if (root.backend) root.backend.previewFirewall(firewallPortField.text.trim())
          }
        }

        Text {
          Layout.fillWidth: true
          visible: root.backend && root.backend.firewallPreview !== ""
          text: root.backend ? root.formatFirewallPreview(root.backend.firewallPreview) : ""
          textFormat: Text.PlainText
          wrapMode: Text.WrapAnywhere
          color: Color.accent
          font.family: root.face
          font.pixelSize: Style.font.caption
        }

        RowLayout {
          Layout.fillWidth: true
          visible: root.backend && root.backend.firewallPreview !== ""
          spacing: Style.space(8)
          Button {
            Layout.fillWidth: true
            text: root.t("Autorizar esta regla")
            bordered: true
            focusable: true
            enabled: root.backend && !root.backend.firewallBusy
            foreground: root.ink
            accent: Color.accent
            fontFamily: root.face
            onClicked: if (root.backend) root.backend.allowFirewall()
          }
          Button {
            text: root.t("Cancelar")
            bordered: true
            focusable: true
            enabled: root.backend && !root.backend.firewallBusy
            foreground: root.ink
            accent: Color.accent
            fontFamily: root.face
            onClicked: if (root.backend) root.backend.cancelFirewall()
          }
        }

        Text {
          Layout.fillWidth: true
          visible: root.backend && (root.backend.firewallMessage !== "" || root.backend.firewallError !== "")
          text: root.backend ? (root.backend.firewallError !== "" ? root.backend.firewallError : root.backend.firewallMessage) : ""
          textFormat: Text.PlainText
          wrapMode: Text.WordWrap
          color: root.backend && root.backend.firewallError !== "" ? Color.urgent : Color.accent
          font.family: root.face
          font.pixelSize: Style.font.caption
        }

        RowLayout {
          Layout.fillWidth: true
          visible: root.backend && !root.backend.available && !root.backend.managedAgentRunning
          spacing: Style.space(8)
          Controls.TextField {
            id: connectAddress
            Layout.fillWidth: true
            placeholderText: root.t("IP del vecino:47832")
            color: root.ink
            font.family: root.face
            background: Rectangle { color: "transparent"; border.color: Color.accent; border.width: 1; radius: 8 }
          }
          Button {
            text: root.t("Conectar por IP")
            bordered: true
            focusable: true
            enabled: root.backend && root.backend.installed && !root.backend.pairingRunning && connectAddress.text.trim() !== ""
            foreground: root.ink
            accent: Color.accent
            fontFamily: root.face
            onClicked: if (root.backend) root.backend.startSender(connectAddress.text.trim())
          }
        }

        Text {
          Layout.fillWidth: true
          visible: root.backend && !root.backend.available && !root.backend.managedAgentRunning
          text: root.t("Conectar por IP sirve cuando el receptor ya está emparejado y ubicado, pero no aparece en Equipos en la red.")
          textFormat: Text.PlainText
          wrapMode: Text.WordWrap
          color: root.muted
          font.family: root.face
          font.pixelSize: Style.font.caption
        }

        RowLayout {
          Layout.fillWidth: true
          visible: root.backend && !root.backend.available && !root.backend.managedAgentRunning
          spacing: Style.space(8)
          Controls.TextField {
            id: meshPort
            Layout.fillWidth: true
            text: "47832"
            placeholderText: root.t("Puerto común de los destinos")
            color: root.ink
            font.family: root.face
            background: Rectangle { color: "transparent"; border.color: Color.accent; border.width: 1; radius: 8 }
          }
          Button {
            text: root.t("Conectar varios equipos")
            bordered: true
            focusable: true
            enabled: root.backend && root.backend.installed && !root.backend.pairingRunning
              && root.placedPeerCount >= 2
              && /^[0-9]{1,5}$/.test(meshPort.text.trim())
              && Number(meshPort.text.trim()) > 0 && Number(meshPort.text.trim()) <= 65535
            foreground: root.ink
            accent: Color.accent
            fontFamily: root.face
            onClicked: if (root.backend) root.backend.startMesh(meshPort.text.trim())
          }
        }

        Text {
          Layout.fillWidth: true
          visible: root.backend && !root.backend.available && !root.backend.managedAgentRunning
          text: root.t("Malla experimental: un ratón controla dos o tres receptores en un mapa 2 × 2. Empareje y ubique todos los equipos; cada receptor debe usar este mismo puerto. Para dos equipos en total, use Conectar arriba.")
          textFormat: Text.PlainText
          wrapMode: Text.WordWrap
          color: root.muted
          font.family: root.face
          font.pixelSize: Style.font.caption
        }

        Button {
          Layout.fillWidth: true
          visible: root.backend && root.backend.managedAgentRunning
          text: root.t("Terminar sesión iniciada desde el panel")
          bordered: true
          focusable: true
          foreground: root.ink
          accent: Color.accent
          fontFamily: root.face
          onClicked: if (root.backend) root.backend.stopManagedAgent()
        }

        Text {
          Layout.fillWidth: true
          text: root.t("1  En el destino, inicie «Recibir control».\n2  En el origen, pulse «Emparejar» junto al destino.\n3  Compare el código en ambos equipos y pulse «Coincide · aprobar aquí» en cada uno.\n4  Ubique el destino junto al origen y pulse «Conectar».\n5  Espere «Listo» y cruce el borde exterior indicado.")
          textFormat: Text.PlainText
          wrapMode: Text.WordWrap
          color: root.muted
          font.family: root.face
          font.pixelSize: Style.font.caption
        }

        Button {
          Layout.fillWidth: true
          visible: root.backend && root.backend.available && root.backend.role === "connect"
          text: root.backend && root.backend.paused ? root.t("Reanudar captura") : root.t("Pausar captura")
          bordered: true
          focusable: true
          foreground: root.ink
          accent: Color.accent
          fontFamily: root.face
          onClicked: if (root.backend) root.backend.togglePause()
        }

        Button {
          Layout.fillWidth: true
          visible: root.backend && root.backend.available && root.backend.role === "serve" && root.backend.phase === "controlling"
          text: root.t("Devolver control al origen")
          bordered: true
          focusable: true
          foreground: root.ink
          accent: Color.accent
          fontFamily: root.face
          onClicked: if (root.backend) root.backend.requestReturn()
        }

        Button {
          Layout.fillWidth: true
          visible: root.backend && root.backend.available && root.backend.role === "serve"
            && !root.backend.paused && (root.backend.phase === "controlling" || root.backend.phase === "connected")
          text: root.t("Cortar entrada remota · emergencia")
          bordered: true
          focusable: true
          foreground: root.ink
          accent: Color.urgent
          fontFamily: root.face
          onClicked: if (root.backend) root.backend.stopRemoteInput()
        }

        Button {
          Layout.fillWidth: true
          visible: root.backend && root.backend.available && root.backend.role === "serve" && root.backend.paused
          text: root.t("Reanudar recepción")
          bordered: true
          focusable: true
          foreground: root.ink
          accent: Color.accent
          fontFamily: root.face
          onClicked: if (root.backend) root.backend.resumeReceiver()
        }

        Text {
          Layout.fillWidth: true
          text: root.t("Escape devuelve el control local desde el origen. Con los mapas configurados en ambos equipos, vuelva por el borde hacia el origen. Si el retorno falla, use «Cortar entrada remota · emergencia» en este receptor; reanude cuando sea seguro.")
          textFormat: Text.PlainText
          wrapMode: Text.WordWrap
          color: Color.accent
          font.family: root.face
          font.pixelSize: Style.font.caption
        }

        PanelSeparator {
          Layout.fillWidth: true
          foreground: root.ink
        }

        PanelSectionHeader {
          Layout.fillWidth: true
          text: root.t("ARCHIVOS · ENTRE EQUIPOS EMPAREJADOS")
          foreground: root.ink
          fontFamily: root.face
        }

        Text {
          Layout.fillWidth: true
          text: root.t("En el destino, elija un directorio y pulse Esperar un archivo. La IP local se elige sola; cada archivo requiere aceptación. Límite predeterminado: 100 MiB.")
          textFormat: Text.PlainText
          wrapMode: Text.WordWrap
          color: root.muted
          font.family: root.face
          font.pixelSize: Style.font.caption
        }

        Controls.TextField {
          id: fileListenAddress
          Layout.fillWidth: true
          placeholderText: root.t("Automático · 47833 (o IP:puerto)")
          color: root.ink
          font.family: root.face
          background: Rectangle { color: "transparent"; border.color: Color.accent; border.width: 1; radius: 8 }
        }

        RowLayout {
          Layout.fillWidth: true
          spacing: Style.space(8)
          Controls.TextField {
            id: fileDirectory
            Layout.fillWidth: true
            placeholderText: root.t("Directorio de destino")
            color: root.ink
            font.family: root.face
            Component.onCompleted: text = Core.StandardPaths.writableLocation(Core.StandardPaths.DownloadLocation)
            background: Rectangle { color: "transparent"; border.color: Color.accent; border.width: 1; radius: 8 }
          }
          Button {
            text: root.t("Elegir")
            bordered: true
            focusable: true
            foreground: root.ink
            accent: Color.accent
            fontFamily: root.face
            onClicked: destinationChooser.open()
          }
        }

        Button {
          Layout.fillWidth: true
          text: root.backend && root.backend.receivingFile ? root.t("Dejar de esperar archivo") : root.t("Esperar un archivo")
          bordered: true
          focusable: true
          enabled: root.backend && (root.backend.receivingFile
            || (root.backend.installed && fileDirectory.text.trim() !== ""))
          foreground: root.ink
          accent: Color.accent
          fontFamily: root.face
          onClicked: {
            if (!root.backend) return
            if (root.backend.receivingFile) root.backend.stopFileReceiver()
            else if (fileListenAddress.text.trim() === "")
              root.backend.startFileReceiverAuto(fileDirectory.text.trim())
            else root.backend.startFileReceiver(fileListenAddress.text.trim(), fileDirectory.text.trim())
          }
        }

        Text {
          Layout.fillWidth: true
          visible: root.backend && root.backend.fileOffer !== null
          text: root.backend && root.backend.fileOffer
            ? root.t("De ") + root.backend.fileOffer.peer + ": " + root.backend.fileOffer.name
              + " (" + root.backend.fileOffer.size + root.t(" bytes)\nSHA-256 ") + root.backend.fileOffer.hash
            : ""
          textFormat: Text.PlainText
          wrapMode: Text.WrapAnywhere
          color: Color.accent
          font.family: root.face
          font.pixelSize: Style.font.caption
        }

        RowLayout {
          Layout.fillWidth: true
          visible: root.backend && root.backend.fileOffer !== null
          spacing: Style.space(8)
          Button {
            Layout.fillWidth: true
            text: root.t("Aceptar archivo")
            bordered: true
            focusable: true
            foreground: root.ink
            accent: Color.accent
            fontFamily: root.face
            onClicked: if (root.backend) root.backend.decideFile(true)
          }
          Button {
            Layout.fillWidth: true
            text: root.t("Rechazar")
            bordered: true
            focusable: true
            foreground: root.ink
            accent: Color.accent
            fontFamily: root.face
            onClicked: if (root.backend) root.backend.decideFile(false)
          }
        }

        Text {
          Layout.fillWidth: true
          text: root.t("En el origen, elija un equipo emparejado y un archivo local.")
          textFormat: Text.PlainText
          wrapMode: Text.WordWrap
          color: root.muted
          font.family: root.face
          font.pixelSize: Style.font.caption
        }

        Controls.TextField {
          id: fileSendAddress
          Layout.fillWidth: true
          placeholderText: root.t("IP del destino:47833")
          color: root.ink
          font.family: root.face
          background: Rectangle { color: "transparent"; border.color: Color.accent; border.width: 1; radius: 8 }
        }

        Repeater {
          model: root.backend ? root.backend.peers.length : 0
          delegate: Button {
            required property int index
            Layout.fillWidth: true
            text: root.t("Enviar a ") + root.backend.peers[index].ip
            bordered: true
            focusable: true
            foreground: root.ink
            accent: Color.accent
            fontFamily: root.face
            onClicked: fileSendAddress.text = root.backend.peers[index].ip + ":47833"
          }
        }

        RowLayout {
          Layout.fillWidth: true
          spacing: Style.space(8)
          Controls.TextField {
            id: fileSourcePath
            Layout.fillWidth: true
            placeholderText: root.t("Ruta absoluta del archivo")
            color: root.ink
            font.family: root.face
            background: Rectangle { color: "transparent"; border.color: Color.accent; border.width: 1; radius: 8 }
          }
          Button {
            text: root.t("Elegir")
            bordered: true
            focusable: true
            foreground: root.ink
            accent: Color.accent
            fontFamily: root.face
            onClicked: sourceChooser.open()
          }
        }

        Button {
          Layout.fillWidth: true
          text: root.backend && root.backend.sendingFile ? root.t("Enviando…") : root.t("Enviar archivo")
          bordered: true
          focusable: true
          enabled: root.backend && root.backend.installed && !root.backend.sendingFile
            && fileSendAddress.text.trim() !== "" && fileSourcePath.text.trim() !== ""
          foreground: root.ink
          accent: Color.accent
          fontFamily: root.face
          onClicked: if (root.backend)
            root.backend.sendFile(fileSendAddress.text.trim(), fileSourcePath.text.trim())
        }

        Text {
          Layout.fillWidth: true
          visible: root.backend && root.backend.fileResult !== ""
          text: root.backend ? root.backend.fileResult : ""
          textFormat: Text.PlainText
          wrapMode: Text.WordWrap
          color: root.ink
          font.family: root.face
          font.pixelSize: Style.font.caption
        }

        Button {
          Layout.fillWidth: true
          text: root.t("Abrir guía completa")
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

        Text {
          Layout.fillWidth: true
          text: "Powered by JhonnySuarez - PuroDelphi"
          textFormat: Text.PlainText
          horizontalAlignment: Text.AlignHCenter
          color: root.muted
          font.family: root.face
          font.pixelSize: Style.font.caption
        }
        }
      }
    }
  }
}
