import QtQuick
import Quickshell.Io

Item {
  id: root
  property bool installed: false
  property bool available: false
  property string role: ""
  property string phase: ""
  readonly property string phaseText: ({
    connecting: "conectando",
    reconnecting: "reconectando",
    pairing: "emparejando",
    ready: "listo",
    controlling: "control remoto",
    handoff: "cediendo control",
    connected: "conectado",
    listening: "disponible",
    locked: "bloqueado",
    paused: "en pausa",
    disconnected: "desconectado"
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
  property bool sendingFile: false
  property var fileOffer: null
  property string fileResult: ""
  property bool stoppingFileReceiver: false
  property bool managedAgentRunning: false
  property bool stoppingManagedAgent: false
  property string lastAgentError: ""

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
  }

  function startFileReceiver(address, directory) {
    if (!installed || receiveFileProcess.running || !address || !directory) return
    error = ""
    fileResult = ""
    fileOffer = null
    stoppingFileReceiver = false
    receiveFileProcess.command = ["seamlesscontrold", "receive-file-ui", address, directory]
    receiveFileProcess.running = true
    receivingFile = true
  }

  function startFileReceiverAuto(directory) {
    if (!installed || receiveFileProcess.running || !directory) return
    error = ""
    fileResult = ""
    fileOffer = null
    stoppingFileReceiver = false
    receiveFileProcess.command = ["seamlesscontrold", "receive-file-auto-ui", "47833", directory]
    receiveFileProcess.running = true
    receivingFile = true
  }

  function stopFileReceiver() {
    if (!receiveFileProcess.running) return
    stoppingFileReceiver = true
    receiveFileProcess.running = false
    receivingFile = false
    fileOffer = null
  }

  function decideFile(accept) {
    if (!receiveFileProcess.running || !fileOffer) return
    receiveFileProcess.write(accept ? "SI\n" : "NO\n")
    fileOffer = null
  }

  function sendFile(address, path) {
    if (!installed || sendFileProcess.running || !address || !path) return
    error = ""
    fileResult = ""
    sendFileProcess.command = ["seamlesscontrold", "send-file", address, path]
    sendFileProcess.running = true
    sendingFile = true
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
    actionName = paused ? "reanudar" : "pausar"
    actionProcess.command = ["seamlesscontrold", paused ? "resume" : "pause"]
    actionProcess.running = true
  }

  function requestReturn() {
    if (actionProcess.running || role !== "serve" || phase !== "controlling") return
    actionName = "devolver el control"
    actionProcess.command = ["seamlesscontrold", "return"]
    actionProcess.running = true
  }

  function stopRemoteInput() {
    if (actionProcess.running || role !== "serve") return
    actionName = "cortar la entrada remota"
    actionProcess.command = ["seamlesscontrold", "emergency-stop"]
    actionProcess.running = true
  }

  function resumeReceiver() {
    if (actionProcess.running || role !== "serve" || !paused) return
    actionName = "reanudar la recepción"
    actionProcess.command = ["seamlesscontrold", "resume"]
    actionProcess.running = true
  }

  function decidePair(accept) {
    if (actionProcess.running || phase !== "pairing") return
    actionName = accept ? "aprobar" : "rechazar"
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
    actionName = "revocar"
    actionProcess.command = ["seamlesscontrold", "revoke", address]
    actionProcess.running = true
  }

  Process {
    id: executableProbe
    command: ["sh", "-c", "command -v seamlesscontrold >/dev/null"]
    onExited: function(code) {
      var wasInstalled = root.installed
      root.installed = code === 0
      if (root.installed && !wasInstalled) {
        root.error = ""
        root.refresh()
        root.refreshPeers()
        root.refreshDiscovery()
        root.refreshTopology()
      } else if (!root.installed) {
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
          root.error = "Respuesta inválida del agente"
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
      root.error = code !== 0 ? "No se pudo " + root.actionName : ""
      root.refresh()
      root.refreshPeers()
      root.refreshTopology()
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
      root.discoveryError = code === 0 ? "" : "Búsqueda local no disponible. Puede usar una IP manual."
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
      if (code !== 0) root.error = "No se pudo guardar la posición del equipo"
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
      if (code !== 0) root.error = root.pairError !== ""
        ? "No se pudo emparejar: " + root.pairError
        : "No se pudo emparejar. Revise IP, red y aprobación en ambos equipos."
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
      if (code !== 0 && !root.stoppingManagedAgent)
        root.error = root.lastAgentError !== "" ? root.lastAgentError
          : "El agente terminó con error. Revise la dirección y la topología."
      root.stoppingManagedAgent = false
      root.refresh()
    }
    onRunningChanged: {
      if (!running && root.managedAgentRunning) {
        root.managedAgentRunning = false
        if (!root.stoppingManagedAgent)
          root.error = "No se pudo mantener el agente en ejecución. Compruebe que está instalado y revise la dirección."
      }
    }
  }

  Process {
    id: receiveFileProcess
    stdinEnabled: true
    stdout: SplitParser {
      onRead: function(line) {
        var fields = String(line).split("\t")
        if (fields.length === 5 && fields[0] === "OFFER") {
          root.fileOffer = {
            peer: fields[1],
            name: fields[2],
            size: Number(fields[3]),
            hash: fields[4]
          }
        } else if (line.indexOf("Archivo guardado en ") === 0) {
          root.fileResult = String(line)
        } else if (line.indexOf("Archivo rechazado") === 0) {
          root.fileResult = "Archivo rechazado o cancelado"
        }
      }
    }
    onExited: function(code) {
      root.receivingFile = false
      root.fileOffer = null
      if (code !== 0 && !root.stoppingFileReceiver)
        root.error = "La recepción de archivos terminó con error. Revise IP, puerto y directorio."
      root.stoppingFileReceiver = false
    }
  }

  Process {
    id: sendFileProcess
    stdout: SplitParser {
      onRead: function(line) { root.fileResult = String(line) }
    }
    onExited: function(code) {
      root.sendingFile = false
      if (code !== 0) root.error = "No se entregó el archivo. Revise el par, la red y la aceptación del destino."
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
