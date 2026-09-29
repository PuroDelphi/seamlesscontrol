import QtQuick
import Quickshell.Io

Item {
  id: root
  property bool available: false
  property string role: ""
  property string phase: ""
  readonly property string phaseText: ({
    connecting: "conectando",
    reconnecting: "reconectando",
    pairing: "emparejando",
    ready: "listo",
    controlling: "control remoto",
    connected: "conectado",
    listening: "disponible",
    paused: "en pausa",
    disconnected: "desconectado"
  })[phase] || phase
  property string peer: ""
  property bool paused: false
  property string pairSas: ""
  property string pairKey: ""
  property string error: ""
  property bool pairingRunning: false
  property string actionName: ""
  property var peers: []
  property var topology: []

  function refresh() {
    if (statusProcess.running) return
    statusProcess.running = true
  }

  function refreshPeers() {
    if (peersProcess.running) return
    peersProcess.running = true
  }

  function refreshTopology() {
    if (topologyProcess.running) return
    topologyProcess.running = true
  }

  function placeMachine(machine, column, row) {
    if (topologyAction.running || !machine) return
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

  function decidePair(accept) {
    if (actionProcess.running || phase !== "pairing") return
    actionName = accept ? "aprobar" : "rechazar"
    actionProcess.command = accept
      ? ["seamlesscontrold", "approve", pairSas]
      : ["seamlesscontrold", "reject"]
    actionProcess.running = true
  }

  function pair(address) {
    if (pairProcess.running || available || !address) return
    error = ""
    pairingRunning = true
    pairProcess.command = ["seamlesscontrold", "pair", address]
    pairProcess.running = true
  }

  function revoke(address) {
    if (actionProcess.running || !address) return
    actionName = "revocar"
    actionProcess.command = ["seamlesscontrold", "revoke", address]
    actionProcess.running = true
  }

  Process {
    id: statusProcess
    command: ["seamlesscontrold", "status"]
    stdout: StdioCollector {
      waitForEnd: true
      onStreamFinished: {
        var line = String(text || "").replace(/\r?\n$/, "")
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
    onExited: function(code) {
      root.pairingRunning = false
      if (code !== 0) root.error = "No se pudo emparejar. Revise IP, red y código."
      root.refresh()
    }
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
}
