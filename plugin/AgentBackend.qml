import QtQuick
import Quickshell
import Quickshell.Io
import "Translations.js" as Tr

Item {
  id: root
  property string language: "en"
  property int fileLimitMiB: 100
  property string approvalMode: "always"
  property int approvalMinutes: 15
  property var approvalUntil: ({})
  property int approvalSecondsRemaining: 0
  property string approvalFeedback: ""
  property string approvalPendingMode: ""
  property int approvalPendingMinutes: 0
  property bool approvalExpiryPending: false
  property bool approvalLoadedOnce: false
  function t(spanish) { return Tr.text(spanish, language) }
  function setLanguage(next) {
    if (next !== "en" && next !== "es") return
    language = next
    languageFile.setText(next + "\n")
  }
  function setFileLimitMiB(value) {
    var number = Number(value)
    if (!Number.isInteger(number) || number < 1 || number > 10240) return false
    fileLimitMiB = number
    fileLimitFile.setText(String(number) + "\n")
    if (clipboardReadProcess.running) clipboardReadProcess.running = false
    if (clipboardReceiveProcess.running && clipboardOffer === null) {
      stoppingClipboardReceiver = true
      clipboardReceiveProcess.running = false
    }
    return true
  }
  function fileCommand(args) {
    return ["env", "SEAMLESSCONTROL_MAX_FILE_BYTES=" + String(fileLimitMiB * 1048576), "seamlesscontrold"].concat(args)
  }
  function setApprovalSettings(mode, minutesText) {
    var minutes = Number(minutesText)
    if (!["always", "automatic", "timed"].includes(mode)
        || !Number.isInteger(minutes) || minutes < 1 || minutes > 1440) {
      approvalFeedback = "error"
      return false
    }
    approvalMode = mode
    approvalMinutes = minutes
    approvalUntil = ({})
    approvalSecondsRemaining = 0
    approvalFeedback = "saving"
    approvalPendingMode = mode
    approvalPendingMinutes = minutes
    try {
      approvalFile.setText(mode + "\t" + String(minutes) + "\n")
      approvalFeedback = approvalExpiryPending ? "expired" : "saved"
    } catch (error) {
      approvalFeedback = "error"
      approvalPendingMode = ""
      approvalExpiryPending = false
      return false
    }
    return true
  }
  function autoAcceptFrom(peer) {
    var machine = peers.find(function(item) { return item.ip === peer })
    return approvalMode === "automatic"
      || approvalMode === "timed" && machine && Number(approvalUntil[machine.key] || 0) > Date.now()
  }
  function rememberApproval(peer) {
    var machine = peers.find(function(item) { return item.ip === peer })
    if (approvalMode === "timed" && machine) {
      var next = Object.assign({}, approvalUntil)
      next[machine.key] = Date.now() + approvalMinutes * 60000
      approvalUntil = next
      approvalSecondsRemaining = approvalMinutes * 60
    }
  }
  property bool installed: false
  property bool available: false
  property string role: ""
  property string phase: ""
  readonly property string phaseText: ({
    connecting: root.t("conectando"),
    reconnecting: root.t("reconectando"),
    pairing: root.t("emparejando"),
    ready: root.t("listo"),
    rearming: root.t("aleja el puntero del borde"),
    controlling: root.t("control remoto"),
    handoff: root.t("cediendo control"),
    connected: root.t("conectado"),
    listening: root.t("disponible"),
    locked: root.t("bloqueado"),
    paused: root.t("en pausa"),
    disconnected: root.t("desconectado")
  })[phase] || phase
  property string peer: ""
  property bool paused: false
  property string pairSas: ""
  property string pairKey: ""
  property string error: ""
  property bool pairingRunning: false
  property string pairError: ""
  property string actionName: ""
  property var peers: []
  property var discovered: []
  property string discoveryError: ""
  property var topology: []
  property bool receivingFile: false
  property bool fileListening: false
  property string fileListenEndpoint: ""
  property string fileReceiveError: ""
  property string fileSendError: ""
  property bool sendingFile: false
  property bool sendingCopiedFile: false
  property string copiedFilePath: ""
  property string lastClipboardSentPath: ""
  property var clipboardOffer: null
  property bool clipboardFileListening: false
  property bool stoppingClipboardReceiver: false
  property string clipboardFileResult: ""
  property string clipboardFileError: ""
  property bool pickerBusy: false
  property string pickerKind: ""
  signal pathChosen(string kind, string path)
  property var fileOffer: null
  property string fileResult: ""
  property string fileError: ""
  property bool stoppingFileReceiver: false
  property bool managedAgentRunning: false
  property bool stoppingManagedAgent: false
  property string lastAgentError: ""
  readonly property string reconnectReason: lastAgentError.replace(/^Conexión interrumpida: /, "")
    .replace(/\. Reintentando en [0-9]+ s\.$/, "")
  property int captureWaitSeconds: 0
  property bool repairBusy: false
  property string repairMessage: ""
  property string repairError: ""
  readonly property string firewallScript: decodeURIComponent(String(Qt.resolvedUrl("../packaging/firewall-lan.sh")).replace(/^file:\/\//, ""))
  readonly property string setupScript: decodeURIComponent(String(Qt.resolvedUrl("../packaging/setup-agent.sh")).replace(/^file:\/\//, ""))
  property bool setupBusy: false
  property string setupMessage: ""
  property string setupError: ""
  property bool firewallBusy: false
  property string firewallPreview: ""
  property string firewallMessage: ""
  property string firewallError: ""
  property string firewallPort: ""
  property string firewallStep: ""
  property string firewallOutput: ""
  property string firewallStderr: ""

  FileView {
    id: languageFile
    path: Quickshell.env("HOME") + "/.config/seamlesscontrol-language"
    watchChanges: true
    atomicWrites: true
    printErrors: false
    onLoaded: root.language = String(text() || "").trim() === "es" ? "es" : "en"
    onFileChanged: reload()
  }

  FileView {
    id: fileLimitFile
    path: Quickshell.env("HOME") + "/.config/seamlesscontrol-file-limit-mib"
    watchChanges: true
    atomicWrites: true
    printErrors: false
    onLoaded: {
      var value = Number(String(text() || "").trim())
      root.fileLimitMiB = Number.isInteger(value) && value >= 1 && value <= 10240 ? value : 100
    }
    onFileChanged: reload()
  }

  FileView {
    id: approvalFile
    path: Quickshell.env("HOME") + "/.config/seamlesscontrol-file-approval"
    watchChanges: true
    atomicWrites: true
    printErrors: false
    onLoaded: {
      var fields = String(text() || "").trim().split("\t")
      var minutes = Number(fields[1])
      var loadedMode = ["always", "automatic", "timed"].includes(fields[0]) ? fields[0] : "always"
      var loadedMinutes = Number.isInteger(minutes) && minutes >= 1 && minutes <= 1440 ? minutes : 15
      if (root.approvalMode !== loadedMode || root.approvalMinutes !== loadedMinutes) {
        root.approvalUntil = ({})
        root.approvalSecondsRemaining = 0
      }
      root.approvalMode = loadedMode
      root.approvalMinutes = loadedMinutes
      if (!root.approvalLoadedOnce) {
        root.approvalLoadedOnce = true
        if (root.approvalMode === "timed") {
          root.approvalMode = "always"
          root.approvalFeedback = "expired"
          approvalFile.setText("always\t" + String(root.approvalMinutes) + "\n")
          return
        }
      }
      if (root.approvalPendingMode !== "") {
        root.approvalFeedback = root.approvalMode === root.approvalPendingMode
          && root.approvalMinutes === root.approvalPendingMinutes
          ? (root.approvalExpiryPending ? "expired" : "saved") : "error"
        root.approvalPendingMode = ""
        root.approvalExpiryPending = false
      }
    }
    onFileChanged: reload()
  }

  function runSetup(action) {
    if (setupProcess.running || (action !== "install" && action !== "remove")) return
    if (action === "remove" && available) {
      setupError = "Termine la sesión antes de retirar el agente."
      return
    }
    setupBusy = true
    if (clipboardReceiveProcess.running) {
      stoppingClipboardReceiver = true
      clipboardReceiveProcess.running = false
    }
    if (clipboardReadProcess.running) clipboardReadProcess.running = false
    setupError = ""
    setupMessage = "Se abrió una terminal de Omarchy. Autorice los cambios allí y vuelva a este panel."
    setupProcess.command = ["omarchy", "launch", "tui",
      "--app-id=io.github.purodelphi.seamlesscontrol.setup", "bash", setupScript, action, language]
    setupProcess.running = true
  }

  Process {
    id: setupProcess
    onExited: function(code) {
      root.setupBusy = false
      if (code !== 0)
        root.setupError = "No se pudo abrir la terminal de Omarchy. Use el comando manual de la guía."
      if (!executableProbe.running) executableProbe.running = true
    }
  }

  function startReceiver(address) {
    if (!installed || agentProcess.running || available || pairingRunning || !address) return
    error = ""
    lastAgentError = ""
    stoppingManagedAgent = false
    agentProcess.command = ["seamlesscontrold", "serve", address]
    agentProcess.running = true
    managedAgentRunning = true
  }

  function startReceiverAuto(port) {
    var number = Number(port)
    if (!installed || agentProcess.running || available || pairingRunning
        || !/^[0-9]{1,5}$/.test(port) || number < 1 || number > 65535) return
    error = ""
    lastAgentError = ""
    stoppingManagedAgent = false
    agentProcess.command = ["seamlesscontrold", "serve-auto", String(number)]
    agentProcess.running = true
    managedAgentRunning = true
  }

  function startSender(address) {
    if (!installed || agentProcess.running || available || pairingRunning || !address) return
    error = ""
    lastAgentError = ""
    repairMessage = ""
    repairError = ""
    stoppingManagedAgent = false
    agentProcess.command = ["seamlesscontrold", "connect", address]
    agentProcess.running = true
    managedAgentRunning = true
  }

  function startMesh(port) {
    var number = Number(port)
    if (!installed || agentProcess.running || available || pairingRunning
        || !/^[0-9]{1,5}$/.test(port) || number < 1 || number > 65535) return
    error = ""
    lastAgentError = ""
    stoppingManagedAgent = false
    agentProcess.command = ["seamlesscontrold", "mesh", String(number)]
    agentProcess.running = true
    managedAgentRunning = true
  }

  function stopManagedAgent() {
    if (!agentProcess.running) return
    stoppingManagedAgent = true
    agentProcess.signal(2)
    stopTimeout.restart()
  }

  function repairCapture() {
    if (repairBusy || !agentProcess.running || role !== "connect"
        || phase !== "connecting" || peer === "" || captureWaitSeconds < 15) return
    repairBusy = true
    repairMessage = ""
    repairError = ""
    stoppingManagedAgent = true
    agentProcess.signal(15)
  }

  function startFileReceiver(address, directory) {
    if (!installed || receiveFileProcess.running || !address || !directory) return
    error = ""
    fileResult = ""
    fileError = ""
    fileOffer = null
    fileListening = false
    fileListenEndpoint = ""
    fileReceiveError = ""
    stoppingFileReceiver = false
    receiveFileProcess.command = fileCommand(["receive-file-ui", address, directory])
    receiveFileProcess.running = true
    receivingFile = true
  }

  function startFileReceiverAuto(directory) {
    if (!installed || receiveFileProcess.running || !directory) return
    error = ""
    fileResult = ""
    fileError = ""
    fileOffer = null
    fileListening = false
    fileListenEndpoint = ""
    fileReceiveError = ""
    stoppingFileReceiver = false
    receiveFileProcess.command = fileCommand(["receive-file-auto-ui", "47833", directory])
    receiveFileProcess.running = true
    receivingFile = true
  }

  function stopFileReceiver() {
    if (!receiveFileProcess.running) return
    stoppingFileReceiver = true
    receiveFileProcess.running = false
    receivingFile = false
    fileListening = false
    fileListenEndpoint = ""
    fileOffer = null
  }

  function decideFile(accept) {
    if (!receiveFileProcess.running || !fileOffer) return
    var peer = fileOffer.peer
    receiveFileProcess.write(accept ? "SI\n" : "NO\n")
    if (accept) rememberApproval(peer)
    fileOffer = null
  }

  function sendFile(address, path, copied) {
    if (!installed || sendFileProcess.running || !address || !path) return
    error = ""
    fileResult = root.t("Preparando archivo…")
    fileError = ""
    fileSendError = ""
    sendingCopiedFile = copied === true
    sendFileProcess.command = fileCommand(["send-file", address, path])
    sendFileProcess.running = true
    sendingFile = true
  }

  function sendCopiedFile(address) {
    if (!copiedFilePath || !address || sendFileProcess.running) return
    lastClipboardSentPath = copiedFilePath
    sendFile(address, copiedFilePath, true)
  }

  function maybeSendCopiedFile() {
    if (copiedFilePath && copiedFilePath !== lastClipboardSentPath
        && peers.length === 1 && !sendFileProcess.running)
      sendCopiedFile(peers[0].ip + ":47834")
  }

  function decideClipboardFile(accept) {
    if (!clipboardReceiveProcess.running || !clipboardOffer) return
    var peer = clipboardOffer.peer
    clipboardReceiveProcess.write(accept ? "SI\n" : "NO\n")
    if (accept) rememberApproval(peer)
    clipboardOffer = null
    if (clipboardNotificationProcess.running) clipboardNotificationProcess.running = false
  }

  function choosePath(kind) {
    if (!installed || pickerBusy || (kind !== "file" && kind !== "folder")) return
    error = ""
    pickerKind = kind
    pickerBusy = true
    pickerProcess.command = ["seamlesscontrold", kind === "file" ? "choose-file" : "choose-folder", language]
    pickerProcess.running = true
  }

  function refresh() {
    if (!installed || statusProcess.running) return
    statusProcess.running = true
  }

  function refreshPeers() {
    if (!installed || peersProcess.running) return
    peersProcess.running = true
  }

  function refreshDiscovery() {
    if (!installed || discoveryProcess.running) return
    discoveryProcess.running = true
  }

  function refreshTopology() {
    if (!installed || topologyProcess.running) return
    topologyProcess.running = true
  }

  function placeMachine(machine, column, row) {
    if (!installed || topologyAction.running || !machine) return
    error = ""
    topologyAction.command = ["seamlesscontrold", "topology", "set", machine, String(column), String(row)]
    topologyAction.running = true
  }

  function togglePause() {
    if (actionProcess.running || role !== "connect") return
    actionName = paused ? root.t("reanudar") : root.t("pausar")
    actionProcess.command = ["seamlesscontrold", paused ? "resume" : "pause"]
    actionProcess.running = true
  }

  function requestReturn() {
    if (actionProcess.running || role !== "serve" || phase !== "controlling") return
    actionName = root.t("devolver el control")
    actionProcess.command = ["seamlesscontrold", "return"]
    actionProcess.running = true
  }

  function stopRemoteInput() {
    if (actionProcess.running || role !== "serve") return
    actionName = root.t("cortar la entrada remota")
    actionProcess.command = ["seamlesscontrold", "emergency-stop"]
    actionProcess.running = true
  }

  function resumeReceiver() {
    if (actionProcess.running || role !== "serve" || !paused) return
    actionName = root.t("reanudar la recepción")
    actionProcess.command = ["seamlesscontrold", "resume"]
    actionProcess.running = true
  }

  function decidePair(accept) {
    if (actionProcess.running || phase !== "pairing") return
    actionName = accept ? root.t("aprobar") : root.t("rechazar")
    actionProcess.command = accept
      ? ["seamlesscontrold", "approve", pairSas]
      : ["seamlesscontrold", "reject"]
    actionProcess.running = true
  }

  function pair(address) {
    if (!installed || pairProcess.running || available || !address) return
    error = ""
    pairError = ""
    pairingRunning = true
    pairProcess.command = ["seamlesscontrold", "pair", address]
    pairProcess.running = true
  }

  function revoke(address) {
    if (!installed || actionProcess.running || !address) return
    actionName = root.t("revocar")
    actionProcess.command = ["seamlesscontrold", "revoke", address]
    actionProcess.running = true
  }

  function previewFirewall(port) {
    var number = Number(port)
    if (firewallProcess.running || !installed) return
    firewallPreview = ""
    firewallMessage = ""
    firewallError = ""
    if (!/^[0-9]{1,5}$/.test(port) || number < 1 || number > 65535) {
      firewallError = root.t("Indique un puerto TCP entre 1 y 65535.")
      return
    }
    firewallPort = String(number)
    firewallStep = "preview"
    firewallOutput = ""
    firewallStderr = ""
    firewallBusy = true
    firewallProcess.command = ["env", "SEAMLESSCONTROL_LANG=" + language,
      "bash", firewallScript, "show", firewallPort]
    firewallProcess.running = true
  }

  function allowFirewall() {
    if (firewallProcess.running || firewallPreview === "" || firewallPort === "") return
    firewallStep = "allow"
    firewallOutput = ""
    firewallStderr = ""
    firewallError = ""
    firewallBusy = true
    firewallProcess.command = ["env", "SEAMLESSCONTROL_LANG=" + language,
      "bash", firewallScript, "allow", firewallPort, "--yes"]
    firewallProcess.running = true
  }

  function cancelFirewall() {
    if (firewallProcess.running) return
    firewallPreview = ""
    firewallPort = ""
  }

  Process {
    id: executableProbe
    command: ["sh", "-c", "command -v seamlesscontrold >/dev/null"]
    onExited: function(code) {
      var wasInstalled = root.installed
      root.installed = code === 0
      if (root.installed && !wasInstalled) {
        root.error = ""
        root.setupMessage = "Agente instalado. Ya puede iniciar una sesión."
        root.refresh()
        root.refreshPeers()
        root.refreshDiscovery()
        root.refreshTopology()
      } else if (!root.installed) {
        if (wasInstalled) root.setupMessage = "Agente retirado."
        root.available = false
        root.peers = []
        root.discovered = []
        root.topology = []
      }
    }
  }

  Process {
    id: statusProcess
    command: ["seamlesscontrold", "status"]
    stdout: StdioCollector {
      waitForEnd: true
      onStreamFinished: {
        var line = String(text || "").replace(/\r?\n$/, "")
        if (line === "") return
        var fields = line.split("\t")
        if (fields.length !== 7 || fields[0] !== "STATUS") {
          root.available = false
          root.error = root.t("Respuesta inválida del agente")
          return
        }
        root.role = fields[1]
        root.phase = fields[2]
        root.peer = fields[3]
        root.paused = fields[4] === "true"
        root.pairSas = fields[5]
        root.pairKey = fields[6]
        root.available = true
      }
    }
    onExited: function(code) {
      if (code !== 0) {
        root.available = false
        root.role = ""
        root.phase = ""
        root.peer = ""
        root.paused = false
        root.pairSas = ""
        root.pairKey = ""
      }
    }
  }

  Process {
    id: actionProcess
    onExited: function(code) {
      root.error = code !== 0 ? root.t("No se pudo ") + root.actionName : ""
      root.refresh()
      root.refreshPeers()
      root.refreshTopology()
    }
  }

  Process {
    id: firewallProcess
    stdout: SplitParser {
      onRead: function(line) { root.firewallOutput = String(line).trim() }
    }
    stderr: SplitParser {
      onRead: function(line) { root.firewallStderr = String(line).trim() }
    }
    onExited: function(code) {
      root.firewallBusy = false
      if (code !== 0) {
        root.firewallError = root.firewallStderr !== "" ? root.firewallStderr
          : root.t("No se pudo configurar UFW; compruebe la autorización y la red local.")
        return
      }
      if (root.firewallStep === "preview") {
        root.firewallPreview = root.firewallOutput
      } else if (root.firewallStep === "allow") {
        root.firewallMessage = root.t("Regla LAN aplicada para TCP ") + root.firewallPort + "."
        root.firewallPreview = ""
      }
    }
  }

  Process {
    id: peersProcess
    command: ["seamlesscontrold", "peers"]
    stdout: StdioCollector {
      waitForEnd: true
      onStreamFinished: {
        var next = []
        String(text || "").split("\n").forEach(function(line) {
          var fields = line.split("\t")
          if (fields.length === 3 && fields[0] === "PEER")
            next.push({ ip: fields[1], key: fields[2] })
        })
        root.peers = next
        root.maybeSendCopiedFile()
      }
    }
  }

  Process {
    id: discoveryProcess
    command: ["seamlesscontrold", "discover"]
    stdout: StdioCollector {
      waitForEnd: true
      onStreamFinished: {
        var next = []
        String(text || "").split("\n").forEach(function(line) {
          var fields = line.split("\t")
          if (fields.length === 5 && fields[0] === "FOUND") {
            var port = Number(fields[3])
            if (port > 0 && port <= 65535)
              next.push({ name: fields[1], ip: fields[2], port: port,
                address: fields[2] + ":" + port, key: fields[4] })
          }
        })
        root.discovered = next
      }
    }
    onExited: function(code) {
      root.discoveryError = code === 0 ? "" : root.t("Búsqueda local no disponible. Puede usar una IP manual.")
      if (code !== 0) root.discovered = []
    }
  }

  Process {
    id: topologyProcess
    command: ["seamlesscontrold", "topology"]
    stdout: StdioCollector {
      waitForEnd: true
      onStreamFinished: {
        var next = []
        String(text || "").split("\n").forEach(function(line) {
          var fields = line.split("\t")
          if (fields.length === 4 && fields[0] === "SLOT")
            next.push({ id: fields[1], column: Number(fields[2]), row: Number(fields[3]) })
        })
        root.topology = next
      }
    }
  }

  Process {
    id: topologyAction
    onExited: function(code) {
      if (code !== 0) root.error = root.t("No se pudo guardar la posición del equipo")
      root.refreshTopology()
    }
  }

  Process {
    id: pairProcess
    stderr: SplitParser {
      onRead: function(line) { root.pairError = String(line).trim() }
    }
    onExited: function(code) {
      root.pairingRunning = false
      if (code !== 0) root.error = /PeerKeyChanged|peer key changed/i.test(root.pairError)
        ? root.t("Esta IP ya está emparejada con otra identidad. Compruebe si otro sistema usa la misma IP; no revoque la identidad anterior si quiere volver a usarla.")
        : root.pairError !== ""
          ? root.t("No se pudo emparejar: ") + root.pairError
          : root.t("No se pudo emparejar. Revise IP, red y aprobación en ambos equipos.")
      root.refresh()
      root.refreshPeers()
      root.refreshTopology()
    }
  }

  Process {
    id: agentProcess
    stdout: SplitParser { onRead: function(line) {} }
    stderr: SplitParser {
      onRead: function(line) { root.lastAgentError = String(line).trim() }
    }
    onExited: function(code) {
      root.managedAgentRunning = false
      stopTimeout.stop()
      if (code !== 0 && !root.stoppingManagedAgent)
        root.error = root.lastAgentError !== "" ? root.lastAgentError
          : root.t("El agente terminó con error. Revise la dirección y la topología.")
      if (root.repairBusy) repairProcess.running = true
      root.stoppingManagedAgent = false
      root.refresh()
    }
    onRunningChanged: {
      if (!running && root.managedAgentRunning) {
        root.managedAgentRunning = false
        if (!root.stoppingManagedAgent)
          root.error = root.t("No se pudo mantener el agente en ejecución. Compruebe que está instalado y revise la dirección.")
      }
    }
  }

  Process {
    id: repairProcess
    command: ["systemctl", "--user", "restart", "xdg-desktop-portal-hyprland.service"]
    onExited: function(code) {
      root.repairBusy = false
      if (code === 0)
        root.repairMessage = root.t("Captura reiniciada. Pulse Conectar otra vez.")
      else
        root.repairError = root.t("No se pudo reiniciar la captura. Revise el servicio del portal de escritorio.")
      root.refresh()
    }
  }

  Timer {
    id: stopTimeout
    interval: 2000
    onTriggered: if (agentProcess.running && root.stoppingManagedAgent) agentProcess.signal(15)
  }

  Timer {
    interval: 1000
    repeat: true
    running: true
    onTriggered: {
      if (root.managedAgentRunning && root.role === "connect"
          && root.phase === "connecting" && root.peer !== "")
        root.captureWaitSeconds++
      else root.captureWaitSeconds = 0
    }
  }

  Process {
    id: receiveFileProcess
    stdinEnabled: true
    stdout: SplitParser {
      onRead: function(line) {
        var fields = String(line).split("\t")
        if (fields.length === 2 && fields[0] === "LISTENING") {
          root.fileListening = true
          root.fileListenEndpoint = fields[1]
        } else if (fields.length === 5 && fields[0] === "OFFER") {
          root.fileOffer = {
            peer: fields[1],
            name: fields[2],
            size: Number(fields[3]),
            hash: fields[4]
          }
          if (root.autoAcceptFrom(fields[1])) {
            receiveFileProcess.write("SI\n")
            root.fileOffer = null
          }
        } else if (line.indexOf("Archivo guardado en ") === 0) {
          root.fileResult = String(line).replace(/^Archivo guardado en /, root.t("Archivo guardado en "))
        } else if (line.indexOf("Archivo rechazado") === 0) {
          root.fileResult = root.t("Archivo rechazado o cancelado")
        }
      }
    }
    stderr: SplitParser {
      onRead: function(line) { root.fileReceiveError = String(line).trim() }
    }
    onExited: function(code) {
      root.receivingFile = false
      root.fileListening = false
      root.fileListenEndpoint = ""
      root.fileOffer = null
      if (code !== 0 && !root.stoppingFileReceiver)
        root.fileError = root.fileReceiveError !== "" ? root.fileReceiveError
          : root.t("La recepción de archivos terminó con error. Revise IP, puerto y directorio.")
      root.stoppingFileReceiver = false
    }
  }

  Process {
    id: pickerProcess
    stdout: StdioCollector {
      waitForEnd: true
      onStreamFinished: {
        var output = String(text || "").trim()
        if (output === "") return
        try {
          var path = JSON.parse(output)
          if (typeof path !== "string" || path.charAt(0) !== "/") throw new Error("invalid path")
          root.pathChosen(root.pickerKind, path)
        } catch (error) {
          root.error = root.t("El selector devolvió una ruta inválida. Escríbala manualmente.")
        }
      }
    }
    onExited: function(code) {
      root.pickerBusy = false
      if (code !== 0) root.error = root.t("No se pudo abrir el selector. Escriba la ruta manualmente.")
    }
  }

  Process {
    id: sendFileProcess
    stdout: SplitParser {
      onRead: function(line) {
        var fields = String(line).split("\t")
        root.fileResult = fields.length === 2 && fields[0] === "PROGRESS"
          ? root.t("Enviando archivo · ") + fields[1] + "%"
          : root.t(String(line))
      }
    }
    stderr: SplitParser {
      onRead: function(line) { root.fileSendError = String(line).trim() }
    }
    onExited: function(code) {
      root.sendingFile = false
      root.sendingCopiedFile = false
      if (code !== 0) {
        root.fileError = /timed out|time.?out/i.test(root.fileSendError)
          ? root.t("Se agotó el tiempo en el puerto de archivos. En el receptor, prepare y autorice su regla LAN para archivos.")
          : root.fileSendError !== "" ? root.fileSendError
          : root.t("No se entregó el archivo. Revise el par, la red y la aceptación del destino.")
      }
    }
  }

  Process {
    id: clipboardReadProcess
    stdout: SplitParser {
      onRead: function(line) {
        try {
          var path = JSON.parse(String(line).trim())
          if (typeof path !== "string" || path.charAt(0) !== "/") {
            if (root.sendingCopiedFile && sendFileProcess.running) sendFileProcess.running = false
            root.copiedFilePath = ""
            root.lastClipboardSentPath = ""
            return
          }
          if (root.sendingCopiedFile && root.copiedFilePath !== path && sendFileProcess.running)
            sendFileProcess.running = false
          root.copiedFilePath = path
          root.lastClipboardSentPath = ""
          root.maybeSendCopiedFile()
        } catch (error) { root.clipboardFileError = root.t("No se pudo leer el archivo copiado.") }
      }
    }
  }

  Process {
    id: clipboardReceiveProcess
    stdinEnabled: true
    stdout: SplitParser {
      onRead: function(line) {
        var fields = String(line).split("\t")
        if (fields.length === 2 && fields[0] === "LISTENING") {
          root.clipboardFileListening = true
          root.clipboardFileError = ""
        } else if (fields.length === 5 && fields[0] === "OFFER") {
          root.clipboardFileResult = ""
          root.clipboardFileError = ""
          root.clipboardOffer = { peer: fields[1], name: fields[2], size: Number(fields[3]), hash: fields[4] }
          if (root.autoAcceptFrom(fields[1])) {
            clipboardReceiveProcess.write("SI\n")
            root.clipboardOffer = null
          } else {
            clipboardNotificationProcess.command = ["notify-send", "--urgency=critical",
              "--app-name=SeamlessControl", "--action=accept=" + root.t("Aceptar"),
              "--action=reject=" + root.t("Rechazar"), root.t("Archivo copiado desde otro equipo"),
              fields[2] + " · " + fields[1] + " · " + fields[3] + root.t(" bytes")]
            clipboardNotificationProcess.running = true
          }
        } else if (fields[0] === "FILE_READY" && fields.length === 2) {
          root.clipboardFileError = ""
          try {
            root.clipboardFileResult = root.t("Archivo listo para pegar: ") + JSON.parse(fields[1])
          } catch (error) { root.clipboardFileResult = root.t("Archivo listo para pegar.") }
          clipboardReadyNotificationProcess.command = ["notify-send", "--app-name=SeamlessControl",
            root.t("Archivo listo para pegar"), root.t("Abra la carpeta de destino y pulse Pegar.")]
          clipboardReadyNotificationProcess.running = true
          root.clipboardOffer = null
        } else if (fields[0] === "FILE_DECLINED") {
          root.clipboardFileResult = root.t("Archivo rechazado o cancelado")
          root.clipboardOffer = null
        } else if (fields.length === 2 && fields[0] === "PROGRESS") {
          root.clipboardFileResult = root.t("Recibiendo archivo · ") + fields[1] + "%"
        } else if (fields[0] === "STAGING_FULL") {
          root.clipboardFileError = root.t("La carpeta temporal de archivos copiados está llena. Libere espacio y vuelva a copiar el archivo.")
        }
      }
    }
    stderr: SplitParser { onRead: function(line) { root.clipboardFileError = String(line).trim() } }
    onExited: function(code) {
      root.clipboardFileListening = false
      root.clipboardOffer = null
      if (clipboardNotificationProcess.running) clipboardNotificationProcess.running = false
      if (code !== 0 && root.installed && !root.stoppingClipboardReceiver && !root.setupBusy)
        root.clipboardFileError = root.t("No se pudo esperar el archivo copiado. Revise el puerto 47834.")
      root.stoppingClipboardReceiver = false
    }
  }

  Process {
    id: clipboardNotificationProcess
    stdout: SplitParser {
      onRead: function(line) {
        if (String(line).trim() === "accept") root.decideClipboardFile(true)
        else if (String(line).trim() === "reject") root.decideClipboardFile(false)
      }
    }
  }

  Process { id: clipboardReadyNotificationProcess }

  Timer {
    interval: 1000
    repeat: true
    running: root.approvalMode === "timed" && Object.keys(root.approvalUntil).length > 0
    onTriggered: {
      var deadlines = Object.keys(root.approvalUntil).map(function(key) {
        return Number(root.approvalUntil[key])
      })
      root.approvalSecondsRemaining = Math.max(0, Math.ceil((Math.max.apply(null, deadlines) - Date.now()) / 1000))
      if (deadlines.some(function(deadline) { return deadline <= Date.now() })) {
        root.approvalExpiryPending = true
        root.setApprovalSettings("always", String(root.approvalMinutes))
      }
    }
  }

  Timer {
    interval: 120000
    repeat: false
    running: root.clipboardOffer !== null
    onTriggered: root.decideClipboardFile(false)
  }

  Timer {
    interval: 4000
    repeat: true
    running: true
    triggeredOnStart: true
    onTriggered: {
      if (!root.installed || root.setupBusy || clipboardReadProcess.running) return
      clipboardReadProcess.command = root.fileCommand(["clipboard-file-watch"])
      clipboardReadProcess.running = true
    }
  }

  Timer {
    interval: 4000
    repeat: true
    running: true
    triggeredOnStart: true
    onTriggered: if (root.installed && !root.setupBusy && !clipboardReceiveProcess.running) {
      clipboardReceiveProcess.command = root.fileCommand(["receive-file-clipboard-ui", "47834"])
      clipboardReceiveProcess.running = true
    }
  }

  Timer {
    interval: 3000
    repeat: true
    running: true
    triggeredOnStart: true
    onTriggered: if (!executableProbe.running) executableProbe.running = true
  }

  Timer {
    interval: 1500
    repeat: true
    running: true
    triggeredOnStart: true
    onTriggered: root.refresh()
  }

  Timer {
    interval: 5000
    repeat: true
    running: true
    triggeredOnStart: true
    onTriggered: root.refreshPeers()
  }

  Timer {
    interval: 5000
    repeat: true
    running: true
    triggeredOnStart: true
    onTriggered: root.refreshTopology()
  }

  Timer {
    interval: 10000
    repeat: true
    running: true
    triggeredOnStart: true
    onTriggered: root.refreshDiscovery()
  }
}
