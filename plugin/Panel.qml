import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as Controls
import QtQuick.Dialogs as Dialogs
import QtCore as Core
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

  function open() { controller.show() }
  function close() { controller.hide() }
  function toggle() { opened ? close() : open() }

  function localPath(url) {
    return decodeURIComponent(String(url).replace(/^file:\/\/(localhost)?/, ""))
  }

  Dialogs.FileDialog {
    id: sourceChooser
    title: "Archivo para SeamlessControl"
    fileMode: Dialogs.FileDialog.OpenFile
    onAccepted: fileSourcePath.text = root.localPath(selectedFile)
  }

  Dialogs.FolderDialog {
    id: destinationChooser
    title: "Guardar archivos de SeamlessControl"
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
          meta: "OMARCHY  ·  " + (root.backend && !root.backend.installed ? "SIN AGENTE"
            : root.backend && root.backend.available ? root.backend.phaseText.toUpperCase() : "SIN SESIÓN")
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

        PanelSectionHeader {
          Layout.fillWidth: true
          text: "EQUIPOS EN LA RED"
          foreground: root.ink
          fontFamily: root.face
        }

        RowLayout {
          Layout.fillWidth: true
          spacing: Style.space(8)
          Text {
            Layout.fillWidth: true
            text: root.backend && root.backend.discoveryError !== ""
              ? root.backend.discoveryError
              : root.backend && root.backend.discovered.length > 0
                ? "Compare el código al emparejar. Una IP nueva se verifica con la clave guardada."
                : "Sin receptores encontrados. Abra Recibir control en el otro Omarchy."
            textFormat: Text.PlainText
            wrapMode: Text.WordWrap
            color: root.muted
            font.family: root.face
            font.pixelSize: Style.font.caption
          }
          Button {
            text: "Buscar"
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
              text: root.changedServerKey(discoveredRow.server) ? "Clave cambió"
                : root.knownServer(discoveredRow.server)
                  ? root.adjacentServer(discoveredRow.server) ? "Compartir" : "Ubicar"
                  : root.previousServerIp(discoveredRow.server) !== "" ? "Actualizar IP" : "Emparejar"
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

        RowLayout {
          Layout.fillWidth: true
          visible: root.backend && !root.backend.available
          spacing: Style.space(8)
          Controls.TextField {
            id: pairAddress
            Layout.fillWidth: true
            placeholderText: "Alternativa manual · IP:puerto"
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
            : root.backend && !root.backend.installed
              ? "Falta el agente. Ejecute bash ~/.config/omarchy/plugins/seamlesscontrol.control/packaging/install-agent.sh; el panel lo detectará automáticamente."
              : "No hay sesión activa. Iníciela desde este panel o desde un terminal."
          textFormat: Text.PlainText
          wrapMode: Text.WordWrap
          color: root.ink
          font.family: root.face
          font.pixelSize: Style.font.body
        }

        PanelSectionHeader {
          Layout.fillWidth: true
          text: "INICIAR SESIÓN"
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
            placeholderText: "Automático · 47832 (o IP:puerto)"
            color: root.ink
            font.family: root.face
            background: Rectangle { color: "transparent"; border.color: Color.accent; border.width: 1; radius: 8 }
          }
          Button {
            text: "Recibir control"
            bordered: true
            focusable: true
            enabled: root.backend && root.backend.installed && !root.backend.pairingRunning
            foreground: root.ink
            accent: Color.accent
            fontFamily: root.face
            onClicked: if (root.backend) {
              var address = listenAddress.text.trim()
              if (address === "") root.backend.startReceiverAuto("47832")
              else root.backend.startReceiver(address)
            }
          }
        }

        RowLayout {
          Layout.fillWidth: true
          visible: root.backend && !root.backend.available && !root.backend.managedAgentRunning
          spacing: Style.space(8)
          Controls.TextField {
            id: connectAddress
            Layout.fillWidth: true
            placeholderText: "IP del vecino:47832"
            color: root.ink
            font.family: root.face
            background: Rectangle { color: "transparent"; border.color: Color.accent; border.width: 1; radius: 8 }
          }
          Button {
            text: "Compartir entrada"
            bordered: true
            focusable: true
            enabled: root.backend && root.backend.installed && !root.backend.pairingRunning && connectAddress.text.trim() !== ""
            foreground: root.ink
            accent: Color.accent
            fontFamily: root.face
            onClicked: if (root.backend) root.backend.startSender(connectAddress.text.trim())
          }
        }

        RowLayout {
          Layout.fillWidth: true
          visible: root.backend && !root.backend.available && !root.backend.managedAgentRunning
          spacing: Style.space(8)
          Controls.TextField {
            id: meshPort
            Layout.fillWidth: true
            text: "47832"
            placeholderText: "Puerto común de los destinos"
            color: root.ink
            font.family: root.face
            background: Rectangle { color: "transparent"; border.color: Color.accent; border.width: 1; radius: 8 }
          }
          Button {
            text: "Compartir en malla"
            bordered: true
            focusable: true
            enabled: root.backend && root.backend.installed && !root.backend.pairingRunning
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
          text: "Malla experimental: empareje y ubique todos los equipos; cada destino debe estar escuchando en el mismo puerto. El origen distribuye el portapapeles de texto entre los destinos conectados."
          textFormat: Text.PlainText
          wrapMode: Text.WordWrap
          color: root.muted
          font.family: root.face
          font.pixelSize: Style.font.caption
        }

        Button {
          Layout.fillWidth: true
          visible: root.backend && root.backend.managedAgentRunning
          text: "Terminar sesión iniciada desde el panel"
          bordered: true
          focusable: true
          foreground: root.ink
          accent: Color.accent
          fontFamily: root.face
          onClicked: if (root.backend) root.backend.stopManagedAgent()
        }

        Text {
          Layout.fillWidth: true
          text: "1  Instale el agente en ambos equipos.\n2  En el destino, inicie «Recibir control».\n3  Empareje y ubique el destino junto al origen en la cuadrícula.\n4  En el origen, inicie «Compartir entrada».\n5  Compare el código de seis cifras en ambos paneles."
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

        Button {
          Layout.fillWidth: true
          visible: root.backend && root.backend.available && root.backend.role === "serve" && root.backend.phase === "controlling"
          text: "Devolver control al origen"
          bordered: true
          focusable: true
          foreground: root.ink
          accent: Color.accent
          fontFamily: root.face
          onClicked: if (root.backend) root.backend.requestReturn()
        }

        Text {
          Layout.fillWidth: true
          text: "Escape devuelve el control local desde el origen. Con los mapas configurados en ambos equipos, vuelva por el borde hacia el origen o use el botón del destino. La prueba física sigue pendiente."
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
          text: "ARCHIVOS · ENTRE EQUIPOS EMPAREJADOS"
          foreground: root.ink
          fontFamily: root.face
        }

        Text {
          Layout.fillWidth: true
          text: "En el destino, elija un directorio y pulse Esperar un archivo. La IP local se elige sola; cada archivo requiere aceptación. Límite predeterminado: 100 MiB."
          textFormat: Text.PlainText
          wrapMode: Text.WordWrap
          color: root.muted
          font.family: root.face
          font.pixelSize: Style.font.caption
        }

        Controls.TextField {
          id: fileListenAddress
          Layout.fillWidth: true
          placeholderText: "Automático · 47833 (o IP:puerto)"
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
            placeholderText: "Directorio de destino"
            color: root.ink
            font.family: root.face
            Component.onCompleted: text = Core.StandardPaths.writableLocation(Core.StandardPaths.DownloadLocation)
            background: Rectangle { color: "transparent"; border.color: Color.accent; border.width: 1; radius: 8 }
          }
          Button {
            text: "Elegir"
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
          text: root.backend && root.backend.receivingFile ? "Dejar de esperar archivo" : "Esperar un archivo"
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
            ? "De " + root.backend.fileOffer.peer + ": " + root.backend.fileOffer.name
              + " (" + root.backend.fileOffer.size + " bytes)\nSHA-256 " + root.backend.fileOffer.hash
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
            text: "Aceptar archivo"
            bordered: true
            focusable: true
            foreground: root.ink
            accent: Color.accent
            fontFamily: root.face
            onClicked: if (root.backend) root.backend.decideFile(true)
          }
          Button {
            Layout.fillWidth: true
            text: "Rechazar"
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
          text: "En el origen, elija un equipo emparejado y un archivo local."
          textFormat: Text.PlainText
          wrapMode: Text.WordWrap
          color: root.muted
          font.family: root.face
          font.pixelSize: Style.font.caption
        }

        Controls.TextField {
          id: fileSendAddress
          Layout.fillWidth: true
          placeholderText: "IP del destino:47833"
          color: root.ink
          font.family: root.face
          background: Rectangle { color: "transparent"; border.color: Color.accent; border.width: 1; radius: 8 }
        }

        Repeater {
          model: root.backend ? root.backend.peers.length : 0
          delegate: Button {
            required property int index
            Layout.fillWidth: true
            text: "Enviar a " + root.backend.peers[index].ip
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
            placeholderText: "Ruta absoluta del archivo"
            color: root.ink
            font.family: root.face
            background: Rectangle { color: "transparent"; border.color: Color.accent; border.width: 1; radius: 8 }
          }
          Button {
            text: "Elegir"
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
          text: root.backend && root.backend.sendingFile ? "Enviando…" : "Enviar archivo"
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
