import QtQuick
import Quickshell.Io

Item {
  id: root
  property bool available: false
  property string role: ""
  property string phase: ""
  property string peer: ""
  property bool paused: false
  property string pairSas: ""
  property string pairKey: ""
  property string error: ""
  property bool pairingRunning: false
  property string actionName: ""

  function refresh() {
    if (statusProcess.running) return
    statusProcess.running = true
  }

  function togglePause() {
    if (actionProcess.running || role !== "connect") return
    actionName = paused ? "reanudar" : "pausar"
    actionProcess.command = ["seamlesscontrold", paused ? "resume" : "pause"]
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
}
