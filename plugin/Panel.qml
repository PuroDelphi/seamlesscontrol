import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as Controls
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
  property string approvalDraft: ""
  property bool confirmRemoveAgent: false
  property string activeTab: "home"
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

  function saveTimedApproval() {
    if (!backend) return
    var value = approvalDraft !== "" ? approvalDraft : approvalMinutesInput.text.trim()
    if (backend.setApprovalSettings("timed", value)) approvalDraft = ""
  }

  function switchTab(tab) {
    if (!["home", "computers", "files", "settings"].includes(tab)) return
    activeTab = tab
    selectedMachine = ""
    keyboardCell = -1
    if (tab !== "settings") {
      confirmRemoveAgent = false
      revokeCandidate = ""
    }
    scroller.contentY = 0
  }

  function openCopiedFirewall() {
    root.switchTab("settings")
    if (root.backend) root.backend.previewFirewall("47834")
    Qt.callLater(function() {
      scroller.contentY = Math.max(0, Math.min(scroller.contentHeight - scroller.height,
        copiedFirewallHeader.mapToItem(content, 0, 0).y - Style.space(12)))
    })
  }

  function formatFirewallPreview(rule) {
    return String(rule || "")
      .replace(/^(Proposed UFW rule: |Regla UFW propuesta: )/, "")
      .replace(" from ", "\nfrom ")
      .replace(" to ", "\nto ")
      .replace(" port ", "\nport ")
      .replace(" proto ", "\nproto ")
      .replace(" comment ", "\ncomment ")
  }

  function fileReceivePort() {
    var address = fileListenAddress.text.trim()
    if (address === "") return "47833"
    var match = address.match(/:([0-9]{1,5})$/)
    return match ? match[1] : ""
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

  function open() {
    switchTab("main")
    controller.show()
  }
  function close() {
    keyboardCell = -1
    selectedMachine = ""
    controller.hide()
  }
  function toggle() { opened ? close() : open() }

  Connections {
    target: root.backend
    function onPathChosen(kind, path) {
      if (kind === "file") fileSourcePath.text = path
      else if (kind === "folder") fileDirectory.text = path
    }
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

    Item {
      id: keyCatcher
      anchors.fill: parent
      focus: true
      Keys.priority: Keys.BeforeItem
      Keys.onPressed: function(event) {
        if (event.key === Qt.Key_Escape) {
          if (root.selectedMachine !== "") root.selectedMachine = ""
          else root.close()
          event.accepted = true
          return
        }
        if (root.activeTab !== "computers" || root.keyboardCell < 0) return
        if (event.key === Qt.Key_Up || event.key === Qt.Key_Down
            || event.key === Qt.Key_Left || event.key === Qt.Key_Right
            || event.text === "h" || event.text === "j" || event.text === "k" || event.text === "l") {
          root.moveKeyboardCell(event.key === Qt.Key_Left || event.text === "h" ? -1
                                : event.key === Qt.Key_Right || event.text === "l" ? 1 : 0,
                                event.key === Qt.Key_Up || event.text === "k" ? -1
                                : event.key === Qt.Key_Down || event.text === "j" ? 1 : 0)
          event.accepted = true
        } else if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter || event.key === Qt.Key_Space) {
          root.activateKeyboardCell()
          event.accepted = true
        }
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
              root.switchTab("computers")
              root.open()
            }
          }
          function onFileOfferChanged() {
            if (root.backend && root.backend.fileOffer !== null) {
              root.switchTab("files")
              root.open()
              Qt.callLater(function() {
                scroller.contentY = Math.max(0, scroller.contentHeight - scroller.height)
              })
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
          StateButton {
            text: "English"
            bordered: true
            focusable: true
            enabled: !!root.backend
            selected: root.backend && root.backend.language === "en"
            foreground: root.ink
            accent: Color.accent
            fontFamily: root.face
            onClicked: if (root.backend) root.backend.setLanguage("en")
          }
          StateButton {
            text: "Español"
            bordered: true
            focusable: true
            enabled: !!root.backend
            selected: root.backend && root.backend.language === "es"
            foreground: root.ink
            accent: Color.accent
            fontFamily: root.face
            onClicked: if (root.backend) root.backend.setLanguage("es")
          }
        }

        GridLayout {
          Layout.fillWidth: true
          columns: 2
          columnSpacing: Style.space(8)
          rowSpacing: Style.space(8)
          StateButton {
            Layout.fillWidth: true
            text: (root.activeTab === "home" ? "● " : "") + root.t("Inicio")
            bordered: true
            focusable: true
            foreground: root.activeTab === "home" ? Color.accent : root.ink
            accent: Color.accent
            fontFamily: root.face
            onClicked: root.switchTab("home")
          }
          StateButton {
            Layout.fillWidth: true
            text: (root.activeTab === "computers" ? "● " : "") + root.t("Equipos")
            bordered: true
            focusable: true
            foreground: root.activeTab === "computers" ? Color.accent : root.ink
            accent: Color.accent
            fontFamily: root.face
            onClicked: root.switchTab("computers")
          }
          StateButton {
            Layout.fillWidth: true
            text: (root.activeTab === "files" ? "● " : "") + root.t("Archivos")
            bordered: true
            focusable: true
            foreground: root.activeTab === "files" ? Color.accent : root.ink
            accent: Color.accent
            fontFamily: root.face
            onClicked: root.switchTab("files")
          }
          StateButton {
            Layout.fillWidth: true
            text: (root.activeTab === "settings" ? "● " : "") + root.t("Ajustes")
            bordered: true
            focusable: true
            foreground: root.activeTab === "settings" ? Color.accent : root.ink
            accent: Color.accent
            fontFamily: root.face
            onClicked: root.switchTab("settings")
          }
        }

        ColumnLayout {
          Layout.fillWidth: true
          visible: root.backend && root.backend.clipboardOffer !== null
          spacing: Style.space(8)
          PanelSectionHeader {
            Layout.fillWidth: true
            text: root.t("ARCHIVO ENTRANTE · DECIDA AHORA")
            foreground: Color.urgent
            fontFamily: root.face
          }
          Text {
            Layout.fillWidth: true
            text: root.backend && root.backend.clipboardOffer
              ? root.backend.clipboardOffer.name + " · " + root.backend.clipboardOffer.peer : ""
            textFormat: Text.PlainText
            wrapMode: Text.WrapAnywhere
            color: root.ink
            font.family: root.face
            font.pixelSize: Style.font.body
          }
          RowLayout {
            Layout.fillWidth: true
            spacing: Style.space(8)
            StateButton {
              Layout.fillWidth: true
              text: root.t("Aceptar archivo")
              bordered: true
              focusable: true
              foreground: root.ink
              accent: Color.accent
              fontFamily: root.face
              onClicked: if (root.backend) root.backend.decideClipboardFile(true)
            }
            StateButton {
              Layout.fillWidth: true
              text: root.t("Rechazar")
              bordered: true
              focusable: true
              foreground: root.ink
              accent: Color.urgent
              fontFamily: root.face
              onClicked: if (root.backend) root.backend.decideClipboardFile(false)
            }
          }
        }

        PanelSectionHeader {
          Layout.fillWidth: true
          visible: root.activeTab === "home"
          text: root.t("PREPARAR ESTE EQUIPO")
          foreground: root.ink
          fontFamily: root.face
        }

        HelpDisclosure {
          Layout.fillWidth: true
          visible: root.activeTab === "home"
          title: root.t("Ayuda · Instalación del agente")
          description: root.backend && root.backend.installed
            ? root.t("El agente ya está instalado. Detenga las sesiones activas antes de actualizarlo. Se abrirá una terminal para mostrar el progreso.")
            : root.t("Después de añadir el plugin con Omarchy, instale aquí el agente y los paquetes que falten. Se abrirá una terminal para mostrar el progreso y pedir autorización si hace falta.")
          foreground: root.ink
          fontFamily: root.face
        }

        PanelSectionHeader {
          Layout.fillWidth: true
          visible: root.activeTab === "settings"
          text: root.t("ADMINISTRAR AGENTE")
          foreground: root.ink
          fontFamily: root.face
        }

        RowLayout {
          Layout.fillWidth: true
          spacing: Style.space(8)
          StateButton {
            Layout.fillWidth: true
            visible: root.activeTab === "home"
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
          StateButton {
            Layout.fillWidth: true
            visible: root.activeTab === "settings"
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
          visible: root.activeTab === "home" && root.backend && root.backend.pluginVersion !== ""
          text: root.backend
            ? root.t("Plugin ") + root.backend.pluginVersion + root.t(" · Agente ")
              + (root.backend.installed ? (root.backend.agentVersion || root.t("comprobando…")) : root.t("sin instalar"))
              + (root.backend.agentVersionMismatch ? root.t(" · VERSIONES DISTINTAS: termine la sesión y actualice el agente.") : "")
            : ""
          textFormat: Text.PlainText
          wrapMode: Text.WordWrap
          color: root.backend && root.backend.agentVersionMismatch ? Color.urgent : root.muted
          font.family: root.face
          font.pixelSize: Style.font.caption
        }

        Text {
          Layout.fillWidth: true
          visible: root.activeTab === "settings" && root.confirmRemoveAgent
          text: root.t("Se retirará el agente y solo los paquetes que instaló SeamlessControl. Las claves y equipos emparejados se conservarán. Después puede quitar el widget con omarchy plugin remove seamlesscontrol.control.")
          textFormat: Text.PlainText
          wrapMode: Text.WordWrap
          color: Color.urgent
          font.family: root.face
          font.pixelSize: Style.font.caption
        }

        RowLayout {
          Layout.fillWidth: true
          visible: root.activeTab === "settings" && root.confirmRemoveAgent
          spacing: Style.space(8)
          StateButton {
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
          StateButton {
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

        StateButton {
          Layout.fillWidth: true
          visible: root.activeTab === "home" && root.backend && root.backend.installed
          text: root.t("Preparar recepción de archivos copiados · TCP 47834 →")
          bordered: true
          focusable: true
          foreground: root.ink
          accent: Color.accent
          fontFamily: root.face
          onClicked: root.openCopiedFirewall()
        }

        ColumnLayout {
          Layout.fillWidth: true
          visible: root.activeTab === "computers"
          spacing: Style.space(12)

        PanelSeparator {
          Layout.fillWidth: true
          foreground: root.ink
        }

        PanelSectionHeader {
          Layout.fillWidth: true
          text: root.t("1 · EMPAREJAR POR DIRECCIÓN")
          foreground: root.ink
          fontFamily: root.face
        }

        Text {
          Layout.fillWidth: true
          text: root.t("Si conoce la IP:puerto del destino, active Recibir control allí y pulse Emparejar. Si no conoce la dirección, use Equipos cercanos en el paso 2. Compare y apruebe el código en ambos equipos.")
          textFormat: Text.PlainText
          wrapMode: Text.WordWrap
          color: root.muted
          font.family: root.face
          font.pixelSize: Style.font.caption
        }

        Text {
          Layout.fillWidth: true
          visible: root.backend && root.backend.available
          text: root.backend && root.backend.role === "serve"
            ? root.t("Este equipo está recibiendo control. Para emparejar desde aquí, pulse Detener recepción y después Emparejar. El otro equipo también puede iniciar el emparejamiento mientras este receptor escucha.")
            : root.t("Este equipo está controlando otro. Termine esa sesión desde Inicio para iniciar un emparejamiento aquí.")
          textFormat: Text.PlainText
          wrapMode: Text.WordWrap
          color: Color.accent
          font.family: root.face
          font.pixelSize: Style.font.caption
        }

        StateButton {
          Layout.fillWidth: true
          visible: root.backend && root.backend.available && root.backend.role === "serve"
          text: root.backend && root.backend.receiverStopRequested
            ? root.t("Deteniendo recepción…") : root.t("Detener recepción para emparejar")
          bordered: true
          focusable: true
          enabled: root.backend && !root.backend.actionRunning && !root.backend.receiverStopRequested
          foreground: root.ink
          accent: Color.accent
          fontFamily: root.face
          onClicked: if (root.backend) root.backend.stopReceiver()
        }

        RowLayout {
          Layout.fillWidth: true
          spacing: Style.space(8)
          Controls.TextField {
            id: pairAddress
            Layout.fillWidth: true
            enabled: root.backend && !root.backend.available
            onTextChanged: if (root.backend) {
              root.backend.diagnosisReason = ""
              root.backend.diagnosisError = ""
            }
            placeholderText: root.t("Dirección del receptor · IP:puerto")
            color: root.ink
            font.family: root.face
            background: Rectangle {
              color: "transparent"
              border.color: Color.accent
              border.width: 1
              radius: 8
            }
          }
          StateButton {
            text: root.backend && root.backend.pairingRunning ? root.t("Conectando…") : root.t("Emparejar")
            bordered: true
            focusable: true
            enabled: root.backend && root.backend.installed && !root.backend.available
              && !root.backend.pairingRunning && pairAddress.text !== ""
            foreground: root.ink
            accent: Color.accent
            fontFamily: root.face
            onClicked: if (root.backend) root.backend.pair(pairAddress.text.trim())
          }
        }

        StateButton {
          Layout.fillWidth: true
          text: root.backend && root.backend.diagnosisBusy ? root.t("Comprobando conexión…") : root.t("Comprobar conexión antes de emparejar")
          bordered: true
          focusable: true
          enabled: root.backend && root.backend.installed && !root.backend.diagnosisBusy
            && pairAddress.text.trim() !== ""
          foreground: root.ink
          accent: Color.accent
          fontFamily: root.face
          onClicked: if (root.backend) root.backend.diagnosePeer(pairAddress.text.trim())
        }

        Text {
          Layout.fillWidth: true
          visible: root.backend && (root.backend.diagnosisReason !== "" || root.backend.diagnosisError !== "")
          text: root.backend ? (root.backend.diagnosisError !== "" ? root.backend.diagnosisError
            : root.backend.diagnosisReason === "control_port_unreachable"
              ? root.t("No responde el puerto de control. En el destino, active Recibir control y autorice el puerto TCP en su firewall; luego compruebe de nuevo.")
            : root.backend.diagnosisReason === "pair_first"
              ? root.t("El puerto responde. Ahora empareje y compare el código en ambos equipos. La respuesta del puerto todavía no verifica la identidad.")
              : root.t("El puerto responde y hay una clave guardada. Puede conectar; la identidad se verificará al iniciar la sesión.")) : ""
          textFormat: Text.PlainText
          wrapMode: Text.WordWrap
          color: root.backend && (root.backend.diagnosisError !== "" || root.backend.diagnosisReason === "control_port_unreachable") ? Color.urgent : Color.accent
          font.family: root.face
          font.pixelSize: Style.font.caption
        }

        Text {
          Layout.fillWidth: true
          visible: root.backend && root.backend.pairingRunning
          text: root.t("Esperando respuesta del receptor. Debe tener Recibir control activo; luego compare y apruebe el código en ambos equipos.")
          textFormat: Text.PlainText
          wrapMode: Text.WordWrap
          color: Color.accent
          font.family: root.face
          font.pixelSize: Style.font.caption
        }

        Text {
          Layout.fillWidth: true
          visible: root.backend && !root.backend.pairingRunning && root.backend.error !== ""
          text: root.backend ? root.backend.error : ""
          textFormat: Text.PlainText
          wrapMode: Text.WordWrap
          color: Color.urgent
          font.family: root.face
          font.pixelSize: Style.font.caption
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
            + root.t("\nCompárelo con el que aparece en el otro equipo y apruébelo en ambas interfaces.")
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
          StateButton {
            Layout.fillWidth: true
            text: root.t("Coincide · aprobar aquí")
            bordered: true
            focusable: true
            foreground: root.ink
            accent: Color.accent
            fontFamily: root.face
            onClicked: if (root.backend) root.backend.decidePair(true)
          }
          StateButton {
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
          text: root.t("2 · EQUIPOS CERCANOS")
          foreground: root.ink
          fontFamily: root.face
        }

        HelpDisclosure {
          Layout.fillWidth: true
          title: root.t("Ayuda · Emparejar y conectar")
          description: root.t("Emparejar autoriza un equipo una sola vez. Conectar inicia cada sesión de control.")
          foreground: root.ink
          fontFamily: root.face
          detailColor: Color.accent
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
                : root.t("Sin receptores descubiertos. En el otro equipo, inicie Recibir control. Si no aparece, use Emparejar por dirección en el paso 1.")
            textFormat: Text.PlainText
            wrapMode: Text.WordWrap
            color: root.muted
            font.family: root.face
            font.pixelSize: Style.font.caption
          }
          StateButton {
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
            StateButton {
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

        }

        ColumnLayout {
          Layout.fillWidth: true
          visible: root.activeTab === "computers"
          spacing: Style.space(12)

        PanelSectionHeader {
          Layout.fillWidth: true
          text: root.t("3 · EQUIPOS EMPAREJADOS")
          foreground: root.ink
          fontFamily: root.face
        }

        Text {
          Layout.fillWidth: true
          text: root.t("Elija qué puede hacer cada identidad emparejada. Control y texto cambian en la próxima conexión; archivos, en la siguiente oferta. Revocar quita la confianza de inmediato.")
          textFormat: Text.PlainText
          wrapMode: Text.WordWrap
          color: root.muted
          font.family: root.face
          font.pixelSize: Style.font.caption
        }

        Text {
          Layout.fillWidth: true
          visible: root.backend && root.backend.peers.length === 0
          text: root.t("Aquí aparecerán los equipos después de aprobar el mismo código en ambos lados.")
          textFormat: Text.PlainText
          wrapMode: Text.WordWrap
          color: root.muted
          font.family: root.face
          font.pixelSize: Style.font.caption
        }

        Repeater {
          model: root.backend ? root.backend.peers.length : 0
          delegate: ColumnLayout {
            id: peerRow
            required property int index
            Layout.fillWidth: true
            spacing: Style.space(8)
            readonly property var peer: root.backend.peers[peerRow.index]
            readonly property var policy: root.backend.peerPolicies[peer.key] || ({ control: true, text: true, files: true, lastConnectedMs: 0 })
            RowLayout {
              Layout.fillWidth: true
              spacing: Style.space(8)
            Text {
              Layout.fillWidth: true
              text: peerRow.peer.ip + " · " + peerRow.peer.key.slice(0, 12) + "…"
              textFormat: Text.PlainText
              color: root.ink
              elide: Text.ElideRight
              font.family: root.face
              font.pixelSize: Style.font.caption
            }
            StateButton {
              text: root.t("Revocar")
              bordered: true
              focusable: true
              foreground: root.ink
              accent: Color.accent
              fontFamily: root.face
              onClicked: root.revokeCandidate = peerRow.peer.ip
            }
            }
            RowLayout {
              Layout.fillWidth: true
              spacing: Style.space(8)
              Repeater {
                model: ["control", "text", "files"]
                delegate: StateButton {
                  required property string modelData
                  text: (peerRow.policy[modelData] ? "● " : "○ ") + root.t(modelData === "control" ? "Control" : modelData === "text" ? "Texto" : "Archivos")
                  bordered: true
                  focusable: true
                  enabled: root.backend && !root.backend.peerPolicyBusy
                  foreground: root.ink
                  accent: Color.accent
                  fontFamily: root.face
                  onClicked: if (root.backend) root.backend.setPeerPermission(peerRow.peer.ip, peerRow.peer.key, modelData, !peerRow.policy[modelData])
                }
              }
            }
            Text {
              Layout.fillWidth: true
              text: peerRow.policy.lastConnectedMs ? root.t("Última conexión: ") + new Date(peerRow.policy.lastConnectedMs).toLocaleString() : root.t("Sin conexión registrada")
              color: root.muted
              font.family: root.face
              font.pixelSize: Style.font.caption
            }
          }
        }

        Text {
          Layout.fillWidth: true
          visible: root.backend && root.backend.peerPolicyError !== ""
          text: root.backend ? root.backend.peerPolicyError : ""
          color: Color.urgent
          font.family: root.face
          font.pixelSize: Style.font.caption
          wrapMode: Text.WordWrap
        }

        RowLayout {
          Layout.fillWidth: true
          visible: root.revokeCandidate !== ""
          spacing: Style.space(8)
          Text {
            Layout.fillWidth: true
            text: root.t("¿Revocar ") + root.revokeCandidate + root.t("? Se bloqueará su clave y se cerrará la sesión. Para emparejar de nuevo, compare y apruebe otro código en ambos equipos.")
            textFormat: Text.PlainText
            wrapMode: Text.WordWrap
            color: Color.urgent
            font.family: root.face
            font.pixelSize: Style.font.caption
          }
          StateButton {
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
          StateButton {
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
          visible: root.backend && root.backend.peers.length > 0
          text: root.t("↓ Seleccione un equipo emparejado arriba y colóquelo junto a ESTE EQUIPO en el mapa.")
          textFormat: Text.PlainText
          wrapMode: Text.WordWrap
          color: Color.accent
          font.family: root.face
          font.pixelSize: Style.font.caption
        }

        PanelSectionHeader {
          Layout.fillWidth: true
          text: root.t("4 · MAPA DE EQUIPOS · 2 × 2")
          foreground: root.ink
          fontFamily: root.face
        }

        HelpDisclosure {
          Layout.fillWidth: true
          title: root.t("Ayuda · Posición y teclado")
          description: root.t("Ubique el otro equipo junto a ESTE EQUIPO en el origen. En una sesión directa, el receptor deduce el borde de regreso del cruce.")
            + "\n\n" + root.t("El mapa guarda la dirección del cruce. Para iniciar la sesión, pulse Conectar en el origen.")
            + "\n\n" + root.t("Teclado: Tab llega al mapa y recorre sus casillas, Enter elige una ficha, las flechas llevan a la casilla de destino y Enter la coloca. Escape cancela la selección.")
          foreground: root.ink
          fontFamily: root.face
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
              activeFocusOnTab: true
              onActiveFocusChanged: {
                if (activeFocus) root.keyboardCell = index
                else if (root.keyboardCell === index) root.keyboardCell = -1
              }
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
            + root.t("\nPara iniciar el control desde este Omarchy, ubique el destino en el mapa.")
          textFormat: Text.PlainText
          wrapMode: Text.WordWrap
          color: Color.accent
          font.family: root.face
          font.pixelSize: Style.font.caption
        }

        Repeater {
          model: root.unassignedPeers.length
          delegate: StateButton {
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

        Text {
          Layout.fillWidth: true
          visible: root.backend && root.backend.available && root.backend.role === "connect"
          text: root.backend && root.backend.phase === "connecting" && root.backend.peer !== ""
            ? root.t("Red conectada. Preparando la captura del ratón y teclado…")
            : root.backend && root.backend.phase === "connecting"
              ? root.t("Conectando con el receptor. Compruebe que allí sigue activo Recibir control.")
            : root.backend && root.backend.phase === "reconnecting"
              ? root.t("Conexión interrumpida. El agente reintenta automáticamente; espere antes de pulsar Conectar otra vez. Si persiste, compruebe que el receptor sigue disponible.")
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
          visible: root.backend && root.backend.role === "connect"
            && root.backend.phase === "reconnecting" && root.backend.reconnectReason !== ""
          text: root.t("Último intento: ") + (root.backend ? root.backend.reconnectReason : "")
          textFormat: Text.PlainText
          wrapMode: Text.WordWrap
          color: Color.urgent
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

        StateButton {
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

        }

        ColumnLayout {
          Layout.fillWidth: true
          visible: root.activeTab === "home"
          spacing: Style.space(12)

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
          StateButton {
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

        }

        ColumnLayout {
          Layout.fillWidth: true
          visible: root.activeTab === "settings"
          spacing: Style.space(12)

        PanelSectionHeader {
          Layout.fillWidth: true
          text: root.t("CRUCE ENTRE PANTALLAS")
          foreground: root.ink
          fontFamily: root.face
        }

        HelpDisclosure {
          Layout.fillWidth: true
          title: root.t("Ayuda · Cruce entre pantallas")
          description: root.t("Fluido cruza al llegar al borde. Deliberado requiere salir del borde y cruzarlo dos veces en 1,6 segundos. Protección a pantalla completa lo exige solo cuando hay una ventana a pantalla completa. Se aplica al iniciar la próxima conexión; Escape y el borde de regreso siguen disponibles.")
          foreground: root.ink
          fontFamily: root.face
        }

        StateButton {
          Layout.fillWidth: true
          text: (root.backend && root.backend.edgePolicy === "fluid" ? "● " : "○ ") + root.t("Cruce fluido")
          bordered: true; focusable: true; foreground: root.ink; accent: Color.accent; fontFamily: root.face
          onClicked: if (root.backend) root.backend.setEdgePolicy("fluid")
        }
        StateButton {
          Layout.fillWidth: true
          text: (root.backend && root.backend.edgePolicy === "deliberate" ? "● " : "○ ") + root.t("Cruce deliberado")
          bordered: true; focusable: true; foreground: root.ink; accent: Color.accent; fontFamily: root.face
          onClicked: if (root.backend) root.backend.setEdgePolicy("deliberate")
        }
        StateButton {
          Layout.fillWidth: true
          text: (root.backend && root.backend.edgePolicy === "fullscreen" ? "● " : "○ ") + root.t("Proteger pantalla completa")
          bordered: true; focusable: true; foreground: root.ink; accent: Color.accent; fontFamily: root.face
          onClicked: if (root.backend) root.backend.setEdgePolicy("fullscreen")
        }
        Text {
          Layout.fillWidth: true
          visible: root.backend && root.backend.edgePolicyFeedback !== ""
          text: root.backend && root.backend.edgePolicyFeedback === "saved" ? root.t("Cruce guardado para la próxima conexión.") : root.t("No se pudo guardar el cruce.")
          color: root.backend && root.backend.edgePolicyFeedback === "error" ? Color.urgent : Color.accent
          font.family: root.face; font.pixelSize: Style.font.caption; wrapMode: Text.WordWrap
        }

        PanelSectionHeader {
          Layout.fillWidth: true
          text: root.t("APROBACIÓN DE ARCHIVOS ENTRANTES")
            + (root.backend && root.backend.approvalFeedback === "saved" ? " · " + root.t("GUARDADO") : "")
          foreground: root.ink
          fontFamily: root.face
        }

        HelpDisclosure {
          Layout.fillWidth: true
          title: root.t("Ayuda · Aprobación de archivos")
          description: root.t("Se aplica a archivos copiados y envíos manuales desde equipos emparejados. En modo temporal, apruebe el primer archivo de cada equipo; los siguientes se aceptan durante el plazo elegido. Al vencer el plazo o reiniciar el plugin, vuelve a Preguntar siempre.")
          foreground: root.ink
          fontFamily: root.face
        }

        StateButton {
          Layout.fillWidth: true
          text: (root.backend && root.backend.approvalMode === "always" ? "● " : "○ ") + root.t("Preguntar siempre")
          bordered: true
          focusable: true
          foreground: root.ink
          accent: Color.accent
          fontFamily: root.face
          onClicked: if (root.backend) root.backend.setApprovalSettings("always", String(root.backend.approvalMinutes))
        }
        StateButton {
          Layout.fillWidth: true
          text: (root.backend && root.backend.approvalMode === "automatic" ? "● " : "○ ") + root.t("Aceptar automáticamente")
          bordered: true
          focusable: true
          foreground: root.ink
          accent: Color.accent
          fontFamily: root.face
          onClicked: if (root.backend) root.backend.setApprovalSettings("automatic", String(root.backend.approvalMinutes))
        }
        RowLayout {
          Layout.fillWidth: true
          spacing: Style.space(8)
          StateButton {
            Layout.fillWidth: true
            text: (root.backend && root.backend.approvalMode === "timed" ? "● " : "○ ") + root.t("Aceptar por un tiempo")
            bordered: true
            focusable: true
            foreground: root.ink
            accent: Color.accent
            fontFamily: root.face
            onClicked: root.saveTimedApproval()
          }
          Controls.TextField {
            id: approvalMinutesInput
            Layout.preferredWidth: 72
            text: root.backend ? String(root.backend.approvalMinutes) : "15"
            inputMethodHints: Qt.ImhDigitsOnly
            validator: IntValidator { bottom: 1; top: 1440 }
            color: root.ink
            font.family: root.face
            background: Rectangle { color: "transparent"; border.color: Color.accent; border.width: 1; radius: 8 }
            onTextEdited: {
              root.approvalDraft = text.trim()
              if (root.backend && root.backend.approvalMode === "timed")
                root.backend.approvalFeedback = "unsaved"
            }
            onEditingFinished: if (root.backend && root.backend.approvalMode === "timed"
                && root.approvalDraft !== "") root.saveTimedApproval()
            onAccepted: root.saveTimedApproval()
          }
          Text {
            text: root.t("min")
            color: root.ink
            font.family: root.face
          }
        }

        StateButton {
          Layout.fillWidth: true
          text: root.t("Guardar aprobación temporal")
          bordered: true
          focusable: true
          foreground: root.ink
          accent: Color.accent
          fontFamily: root.face
          onClicked: root.saveTimedApproval()
        }

        Text {
          Layout.fillWidth: true
          visible: root.backend && root.backend.approvalFeedback !== ""
          text: !root.backend ? "" : root.backend.approvalFeedback === "saved"
            ? root.t("Modo de aprobación guardado.") + " · " + root.backend.approvalMinutes + " " + root.t("min")
            : root.backend.approvalFeedback === "saving"
              ? root.t("Guardando modo de aprobación…")
              : root.backend.approvalFeedback === "unsaved"
                ? root.t("Tiempo sin guardar.")
              : root.backend.approvalFeedback === "expired"
                ? root.t("Terminó el plazo. Se preguntará por cada archivo.")
                : root.t("No se pudo guardar el modo de aprobación. Revise el tiempo elegido.")
          color: root.backend && root.backend.approvalFeedback === "error" ? Color.urgent : Color.accent
          font.family: root.face
          font.pixelSize: Style.font.caption
          wrapMode: Text.WordWrap
        }

        Text {
          Layout.fillWidth: true
          visible: root.backend && root.backend.approvalMode === "timed"
            && root.backend.approvalSecondsRemaining > 0
          text: !root.backend ? "" : root.t("Permiso temporal activo: ")
            + Math.floor(root.backend.approvalSecondsRemaining / 60) + ":"
            + (root.backend.approvalSecondsRemaining % 60 < 10 ? "0" : "")
            + (root.backend.approvalSecondsRemaining % 60)
          color: Color.accent
          font.family: root.face
          font.pixelSize: Style.font.caption
        }

        Text {
          Layout.fillWidth: true
          text: root.t("Solo los equipos ya emparejados pueden enviar archivos. Al vencer el plazo, el ajuste vuelve a Preguntar siempre. Active el modo temporal otra vez si lo desea.")
          textFormat: Text.PlainText
          wrapMode: Text.WordWrap
          color: root.muted
          font.family: root.face
          font.pixelSize: Style.font.caption
        }

        }

        ColumnLayout {
          Layout.fillWidth: true
          visible: root.activeTab === "settings"
          spacing: Style.space(12)

        PanelSectionHeader {
          Layout.fillWidth: true
          visible: root.backend && root.backend.installed
          text: root.t("FIREWALL · SOLO EN EL RECEPTOR")
          foreground: root.ink
          fontFamily: root.face
        }

        HelpDisclosure {
          Layout.fillWidth: true
          visible: root.backend && root.backend.installed
          title: root.t("Ayuda · Puerto de control")
          description: root.t("Si el otro equipo agota el tiempo de conexión, permita aquí el mismo puerto TCP elegido para «Recibir control». Primero verá la regla limitada a esta red local; Omarchy pedirá autorización antes de aplicarla.")
          foreground: root.ink
          fontFamily: root.face
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
          StateButton {
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
            && root.backend.firewallPort === firewallPortField.text.trim()
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
            && root.backend.firewallPort === firewallPortField.text.trim()
          spacing: Style.space(8)
          StateButton {
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
          StateButton {
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
          visible: root.backend && root.backend.firewallPort === firewallPortField.text.trim()
            && (root.backend.firewallMessage !== "" || root.backend.firewallError !== "")
          text: root.backend ? (root.backend.firewallError !== "" ? root.backend.firewallError : root.backend.firewallMessage) : ""
          textFormat: Text.PlainText
          wrapMode: Text.WordWrap
          color: root.backend && root.backend.firewallError !== "" ? Color.urgent : Color.accent
          font.family: root.face
          font.pixelSize: Style.font.caption
        }

        }

        ColumnLayout {
          Layout.fillWidth: true
          visible: root.activeTab === "settings"
          spacing: Style.space(12)

        PanelSectionHeader {
          id: copiedFirewallHeader
          Layout.fillWidth: true
          text: root.t("ARCHIVOS COPIADOS · PREPARAR RECEPCIÓN")
          foreground: root.ink
          fontFamily: root.face
        }

        HelpDisclosure {
          Layout.fillWidth: true
          title: root.t("Ayuda · Puerto de archivos copiados")
          description: root.t("Autorice TCP 47834 una vez en este receptor para recibir archivos copiados desde equipos emparejados. Después podrá aceptar o rechazar cada archivo desde la notificación, incluso en otro workspace.")
          foreground: root.ink
          fontFamily: root.face
        }

        StateButton {
          Layout.fillWidth: true
          text: root.t("Preparar regla LAN para pegar archivos · 47834")
          bordered: true
          focusable: true
          enabled: root.backend && root.backend.installed && !root.backend.firewallBusy
          foreground: root.ink
          accent: Color.accent
          fontFamily: root.face
          onClicked: if (root.backend) root.backend.previewFirewall("47834")
        }

        Text {
          Layout.fillWidth: true
          visible: root.backend && root.backend.firewallPreview !== "" && root.backend.firewallPort === "47834"
          text: root.backend ? root.formatFirewallPreview(root.backend.firewallPreview) : ""
          textFormat: Text.PlainText
          wrapMode: Text.WrapAnywhere
          color: Color.accent
          font.family: root.face
          font.pixelSize: Style.font.caption
        }

        StateButton {
          Layout.fillWidth: true
          visible: root.backend && root.backend.firewallPreview !== "" && root.backend.firewallPort === "47834"
          text: root.t("Autorizar esta regla")
          bordered: true
          focusable: true
          enabled: root.backend && !root.backend.firewallBusy
          foreground: root.ink
          accent: Color.accent
          fontFamily: root.face
          onClicked: if (root.backend) root.backend.allowFirewall()
        }

        Text {
          Layout.fillWidth: true
          visible: root.backend && root.backend.firewallPort === "47834"
            && (root.backend.firewallMessage !== "" || root.backend.firewallError !== "")
          text: root.backend ? (root.backend.firewallError !== "" ? root.backend.firewallError : root.backend.firewallMessage) : ""
          textFormat: Text.PlainText
          wrapMode: Text.WordWrap
          color: root.backend && root.backend.firewallError !== "" ? Color.urgent : Color.accent
          font.family: root.face
          font.pixelSize: Style.font.caption
        }

        PanelSectionHeader {
          Layout.fillWidth: true
          text: root.t("OPCIONES AVANZADAS")
          foreground: root.ink
          fontFamily: root.face
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
          StateButton {
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

        HelpDisclosure {
          Layout.fillWidth: true
          visible: root.backend && !root.backend.available && !root.backend.managedAgentRunning
          title: root.t("Ayuda · Conexión por IP")
          description: root.t("Conectar por IP sirve cuando el receptor ya está emparejado y ubicado, pero no aparece en Equipos en la red.")
          foreground: root.ink
          fontFamily: root.face
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
          StateButton {
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

        HelpDisclosure {
          Layout.fillWidth: true
          visible: root.backend && !root.backend.available && !root.backend.managedAgentRunning
          title: root.t("Ayuda · Varios equipos")
          description: root.t("Malla experimental: un ratón controla dos o tres receptores en un mapa 2 × 2. Empareje y ubique todos los equipos; cada receptor debe usar este mismo puerto. Para dos equipos en total, use Conectar arriba.")
          foreground: root.ink
          fontFamily: root.face
        }

        }

        ColumnLayout {
          Layout.fillWidth: true
          visible: root.activeTab === "home"
          spacing: Style.space(12)

        StateButton {
          Layout.fillWidth: true
          visible: root.backend && root.backend.managedAgentRunning && root.backend.role !== "serve"
          text: root.t("Terminar sesión iniciada desde el panel")
          bordered: true
          focusable: true
          foreground: root.ink
          accent: Color.accent
          fontFamily: root.face
          onClicked: if (root.backend) root.backend.stopManagedAgent()
        }

        StateButton {
          Layout.fillWidth: true
          visible: root.backend && root.backend.available && root.backend.role === "serve"
          text: root.backend && root.backend.receiverStopRequested
            ? root.t("Deteniendo recepción…") : root.t("Detener recepción")
          bordered: true
          focusable: true
          enabled: root.backend && !root.backend.actionRunning && !root.backend.receiverStopRequested
          foreground: root.ink
          accent: Color.accent
          fontFamily: root.face
          onClicked: if (root.backend) root.backend.stopReceiver()
        }

        HelpDisclosure {
          Layout.fillWidth: true
          title: root.t("Ayuda · Conexión paso a paso")
          description: root.t("1  En el destino, inicie «Recibir control».\n2  En el origen, pulse «Emparejar» junto al destino.\n3  Compare el código en ambos equipos y pulse «Coincide · aprobar aquí» en cada uno.\n4  Ubique el destino junto al origen y pulse «Conectar».\n5  Espere «Listo» y cruce el borde exterior indicado.")
          foreground: root.ink
          fontFamily: root.face
        }

        StateButton {
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

        StateButton {
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

        StateButton {
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

        StateButton {
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

        HelpDisclosure {
          Layout.fillWidth: true
          title: root.t("Ayuda · Volver al origen")
          description: root.t("Escape devuelve el control local desde el origen. Con los mapas configurados en ambos equipos, vuelva por el borde hacia el origen. Si el retorno falla, use «Cortar entrada remota · emergencia» en este receptor; reanude cuando sea seguro.")
          foreground: root.ink
          fontFamily: root.face
          detailColor: Color.accent
        }

        }

        ColumnLayout {
          Layout.fillWidth: true
          visible: root.activeTab === "files"
          spacing: Style.space(12)

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

        HelpDisclosure {
          Layout.fillWidth: true
          title: root.t("Ayuda · Copiar y pegar archivos")
          description: root.t("Copie un archivo, varios archivos o una carpeta en el explorador. Con un solo equipo emparejado se ofrecen automáticamente; con varios, elija el destino aquí. El receptor aprueba una sola oferta para todo el grupo. Después de la verificación, use Pegar en su explorador. El receptor debe permitir TCP 47834 en la LAN.")
          foreground: root.ink
          fontFamily: root.face
        }

        Text {
          Layout.fillWidth: true
          text: root.backend && root.backend.copiedFilePath !== ""
            ? root.t("Selección copiada: ") + root.backend.copiedFilePath.split("/").pop()
            : root.t("Copie archivos o una carpeta en el explorador para ofrecerlos.")
          textFormat: Text.PlainText
          wrapMode: Text.WrapAnywhere
          color: root.ink
          font.family: root.face
          font.pixelSize: Style.font.caption
        }

        Text {
          Layout.fillWidth: true
          visible: root.backend && (root.backend.clipboardFileResult !== "" || root.backend.clipboardFileError !== "")
          text: root.backend ? (root.backend.clipboardFileError !== "" ? root.backend.clipboardFileError : root.backend.clipboardFileResult) : ""
          textFormat: Text.PlainText
          wrapMode: Text.WrapAnywhere
          color: root.backend && root.backend.clipboardFileError !== "" ? Color.urgent : Color.accent
          font.family: root.face
          font.pixelSize: Style.font.caption
        }

        Repeater {
          model: root.backend && root.backend.copiedFilePath !== "" ? root.backend.peers.length : 0
          delegate: StateButton {
            required property int index
            Layout.fillWidth: true
            text: root.t("Ofrecer archivo copiado a ") + root.backend.peers[index].ip
            bordered: true
            focusable: true
            enabled: !root.backend.sendingFile
            foreground: root.ink
            accent: Color.accent
            fontFamily: root.face
            onClicked: root.backend.sendCopiedFile(root.backend.peers[index].ip + ":47834")
          }
        }

        Text {
          Layout.fillWidth: true
          text: root.backend && root.backend.clipboardOffer
            ? root.t("Archivo entrante de ") + root.backend.clipboardOffer.peer + ": "
              + root.backend.clipboardOffer.name + " (" + root.backend.clipboardOffer.size + root.t(" bytes)")
            : root.backend && root.backend.clipboardFileListening
              ? root.t("Disponible para archivos copiados · TCP 47834")
              : root.t("Preparando recepción de archivos copiados…")
          textFormat: Text.PlainText
          wrapMode: Text.WordWrap
          color: Color.accent
          font.family: root.face
          font.pixelSize: Style.font.caption
        }

        RowLayout {
          Layout.fillWidth: true
          visible: root.backend && root.backend.clipboardOffer !== null
          spacing: Style.space(8)
          StateButton {
            Layout.fillWidth: true
            text: root.t("Aceptar archivo")
            bordered: true
            focusable: true
            foreground: root.ink
            accent: Color.accent
            fontFamily: root.face
            onClicked: if (root.backend) root.backend.decideClipboardFile(true)
          }
          StateButton {
            Layout.fillWidth: true
            text: root.t("Rechazar")
            bordered: true
            focusable: true
            foreground: root.ink
            accent: Color.accent
            fontFamily: root.face
            onClicked: if (root.backend) root.backend.decideClipboardFile(false)
          }
        }

        HelpDisclosure {
          Layout.fillWidth: true
          title: root.t("Ayuda · Recibir archivos")
          description: root.t("En el destino, elija un directorio y pulse Esperar un archivo. La IP local se elige sola; cada archivo requiere aceptación. Ajuste abajo el tamaño máximo por equipo.")
          foreground: root.ink
          fontFamily: root.face
        }

        Text {
          Layout.fillWidth: true
          text: root.t("Tamaño máximo de archivo · MiB")
          color: root.ink
          font.family: root.face
          font.pixelSize: Style.font.caption
        }

        RowLayout {
          Layout.fillWidth: true
          spacing: Style.space(8)
          Controls.TextField {
            id: fileLimitInput
            Layout.fillWidth: true
            text: root.backend ? String(root.backend.fileLimitMiB) : "100"
            inputMethodHints: Qt.ImhDigitsOnly
            validator: IntValidator { bottom: 1; top: 10240 }
            color: root.ink
            font.family: root.face
            background: Rectangle { color: "transparent"; border.color: Color.accent; border.width: 1; radius: 8 }
            onAccepted: if (root.backend) root.backend.setFileLimitMiB(text.trim())
          }
          StateButton {
            text: root.t("Guardar límite")
            bordered: true
            focusable: true
            enabled: root.backend && /^\d+$/.test(fileLimitInput.text.trim())
              && Number(fileLimitInput.text) >= 1 && Number(fileLimitInput.text) <= 10240
            foreground: root.ink
            accent: Color.accent
            fontFamily: root.face
            onClicked: if (root.backend) root.backend.setFileLimitMiB(fileLimitInput.text.trim())
          }
        }

        Text {
          Layout.fillWidth: true
          text: root.t("De 1 MiB a 10 GiB (10.240 MiB); valor inicial: 100 MiB. Se aplica al enviar y al comenzar a esperar. Si ya espera un archivo, detenga y reinicie la espera.")
          textFormat: Text.PlainText
          wrapMode: Text.WordWrap
          color: root.ink
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
          StateButton {
            text: root.t("Elegir")
            bordered: true
            focusable: true
            enabled: root.backend && root.backend.installed && !root.backend.pickerBusy
            foreground: root.ink
            accent: Color.accent
            fontFamily: root.face
            onClicked: if (root.backend) root.backend.choosePath("folder")
          }
        }

        StateButton {
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
          visible: root.backend && root.backend.receivingFile
          text: root.backend && root.backend.fileListening
            ? root.t("Esperando archivo en ") + root.backend.fileListenEndpoint
            : root.t("Preparando la recepción de archivos…")
          textFormat: Text.PlainText
          wrapMode: Text.WordWrap
          color: Color.accent
          font.family: root.face
          font.pixelSize: Style.font.caption
        }

        HelpDisclosure {
          Layout.fillWidth: true
          title: root.t("Ayuda · Firewall para archivos")
          description: root.t("ANTES DEL PRIMER ENVÍO: en este receptor, prepare y autorice la regla LAN para el puerto de archivos 47833 (o el puerto que eligió arriba). El puerto del control, 47832 por defecto, no sirve para archivos. Vuelva a pulsar Esperar un archivo para cada envío.")
          foreground: root.ink
          fontFamily: root.face
        }

        StateButton {
          Layout.fillWidth: true
          text: root.t("Preparar regla LAN para archivos")
          bordered: true
          focusable: true
          enabled: root.backend && root.backend.installed && !root.backend.firewallBusy
            && root.fileReceivePort() !== ""
          foreground: root.ink
          accent: Color.accent
          fontFamily: root.face
          onClicked: if (root.backend) root.backend.previewFirewall(root.fileReceivePort())
        }

        Text {
          Layout.fillWidth: true
          visible: root.backend && root.backend.firewallPreview !== ""
            && root.backend.firewallPort === root.fileReceivePort()
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
            && root.backend.firewallPort === root.fileReceivePort()
          spacing: Style.space(8)
          StateButton {
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
          StateButton {
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
          visible: root.backend && root.backend.firewallPort === root.fileReceivePort()
            && (root.backend.firewallMessage !== "" || root.backend.firewallError !== "")
          text: root.backend ? (root.backend.firewallError !== "" ? root.backend.firewallError : root.backend.firewallMessage) : ""
          textFormat: Text.PlainText
          wrapMode: Text.WordWrap
          color: root.backend && root.backend.firewallError !== "" ? Color.urgent : Color.accent
          font.family: root.face
          font.pixelSize: Style.font.caption
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
          StateButton {
            Layout.fillWidth: true
            text: root.t("Aceptar archivo")
            bordered: true
            focusable: true
            foreground: root.ink
            accent: Color.accent
            fontFamily: root.face
            onClicked: if (root.backend) root.backend.decideFile(true)
          }
          StateButton {
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

        HelpDisclosure {
          Layout.fillWidth: true
          title: root.t("Ayuda · Enviar archivos")
          description: root.t("En el origen, elija un equipo emparejado y un archivo local.")
          foreground: root.ink
          fontFamily: root.face
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
          delegate: StateButton {
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
          StateButton {
            text: root.t("Elegir")
            bordered: true
            focusable: true
            enabled: root.backend && root.backend.installed && !root.backend.pickerBusy
            foreground: root.ink
            accent: Color.accent
            fontFamily: root.face
            onClicked: if (root.backend) root.backend.choosePath("file")
          }
        }

        StateButton {
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

        Text {
          Layout.fillWidth: true
          visible: root.backend && root.backend.fileError !== ""
          text: root.backend ? root.backend.fileError : ""
          textFormat: Text.PlainText
          wrapMode: Text.WordWrap
          color: Color.urgent
          font.family: root.face
          font.pixelSize: Style.font.caption
        }

        }

        StateButton {
          Layout.fillWidth: true
          visible: root.activeTab === "settings"
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
