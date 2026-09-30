#!/usr/bin/env python3
"""One connection: expose a receiver at a new IP and change the source IP."""

import socket
import sys
import threading


def relay(source: socket.socket, destination: socket.socket) -> None:
    try:
        while data := source.recv(65536):
            destination.sendall(data)
    finally:
        try:
            destination.shutdown(socket.SHUT_WR)
        except OSError:
            pass


def main() -> None:
    listen_port, target_port = map(int, sys.argv[1:3])
    with socket.socket() as listener:
        listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        listener.bind(("127.0.0.3", listen_port))
        listener.listen(1)
        print("READY", flush=True)
        incoming, _ = listener.accept()
        with incoming, socket.socket() as outgoing:
            outgoing.bind(("127.0.0.2", 0))
            outgoing.connect(("127.0.0.1", target_port))
            forward = threading.Thread(target=relay, args=(incoming, outgoing))
            forward.start()
            relay(outgoing, incoming)
            forward.join()


if __name__ == "__main__":
    main()
