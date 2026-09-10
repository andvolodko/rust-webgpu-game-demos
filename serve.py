#!/usr/bin/env python3
"""HTTP(S) сервер для WASM-демо (Arkanoid, Tanks).
Слухає всі інтерфейси (0.0.0.0), щоб можна було відкрити з LAN.
WebGPU з іншого пристрою потребує HTTPS — самопідписаний сертифікат
генерується автоматично, якщо є openssl.

  python serve.py          # :8000 HTTP + :8443 HTTPS
  python serve.py 9000
"""
from __future__ import annotations

import http.server
import os
import socket
import socketserver
import ssl
import subprocess
import sys
import threading

PORT = int(sys.argv[1]) if len(sys.argv) > 1 else 8000
HTTPS_PORT = PORT + 443 if PORT == 8000 else PORT + 1
HOST = "0.0.0.0"
CERT_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), ".dev-certs")
CERT_FILE = os.path.join(CERT_DIR, "cert.pem")
KEY_FILE = os.path.join(CERT_DIR, "key.pem")


class Handler(http.server.SimpleHTTPRequestHandler):
    extensions_map = {
        **http.server.SimpleHTTPRequestHandler.extensions_map,
        ".wasm": "application/wasm",
        ".js": "text/javascript",
    }

    def end_headers(self):
        self.send_header("Cache-Control", "no-store")
        super().end_headers()

    def log_message(self, fmt, *args):
        sys.stderr.write("%s - %s\n" % (self.address_string(), fmt % args))


class Server(socketserver.ThreadingTCPServer):
    allow_reuse_address = True
    daemon_threads = True


def local_ips() -> list[str]:
    found: list[str] = []
    try:
        with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as s:
            s.connect(("8.8.8.8", 80))
            ip = s.getsockname()[0]
            if ip and not ip.startswith("127."):
                found.append(ip)
    except OSError:
        pass
    try:
        for info in socket.getaddrinfo(socket.gethostname(), None, socket.AF_INET):
            ip = info[4][0]
            if ip not in found and not ip.startswith("127."):
                found.append(ip)
    except OSError:
        pass
    return found


def openssl_bin() -> str | None:
    for name in ("openssl", r"C:\Program Files\Git\usr\bin\openssl.exe"):
        try:
            subprocess.run(
                [name, "version"],
                check=True,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
            return name
        except (OSError, subprocess.CalledProcessError):
            continue
    return None


def ensure_cert(ips: list[str]) -> bool:
    if os.path.isfile(CERT_FILE) and os.path.isfile(KEY_FILE):
        return True
    openssl = openssl_bin()
    if not openssl:
        return False
    os.makedirs(CERT_DIR, exist_ok=True)
    sans = ["DNS:localhost", "IP:127.0.0.1"]
    for ip in ips:
        sans.append(f"IP:{ip}")
    san = ",".join(sans)
    cmd = [
        openssl,
        "req",
        "-x509",
        "-newkey",
        "rsa:2048",
        "-keyout",
        KEY_FILE,
        "-out",
        CERT_FILE,
        "-days",
        "365",
        "-nodes",
        "-subj",
        "/CN=CrystalArkanoid",
        "-addext",
        f"subjectAltName={san}",
    ]
    try:
        subprocess.run(cmd, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        return os.path.isfile(CERT_FILE) and os.path.isfile(KEY_FILE)
    except (OSError, subprocess.CalledProcessError):
        return False


def serve(httpd: Server) -> None:
    try:
        httpd.serve_forever()
    except KeyboardInterrupt:
        pass


def print_urls(ips: list[str], https: bool) -> None:
    print(f"HTTP  http://127.0.0.1:{PORT}", flush=True)
    for ip in ips:
        print(f"      http://{ip}:{PORT}", flush=True)
    if https:
        print(
            f"HTTPS https://127.0.0.1:{HTTPS_PORT}  (self-signed cert — click Advanced/Proceed)",
            flush=True,
        )
        for ip in ips:
            print(f"      https://{ip}:{HTTPS_PORT}", flush=True)
        print("From another device open HTTPS, otherwise WebGPU will not start.", flush=True)
    else:
        print(
            "No HTTPS (openssl not found). LAN HTTP may fail WebGPU (not a secure context).",
            flush=True,
        )
        print(f"Local: http://127.0.0.1:{PORT} still works.", flush=True)
    print("Ctrl+C to stop", flush=True)


if __name__ == "__main__":
    ips = local_ips()
    https_ok = ensure_cert(ips)
    print_urls(ips, https_ok)

    http_httpd = Server((HOST, PORT), Handler)
    threads = [threading.Thread(target=serve, args=(http_httpd,), daemon=True)]

    https_httpd = None
    if https_ok:
        ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        ctx.load_cert_chain(CERT_FILE, KEY_FILE)
        https_httpd = Server((HOST, HTTPS_PORT), Handler)
        https_httpd.socket = ctx.wrap_socket(https_httpd.socket, server_side=True)
        threads.append(threading.Thread(target=serve, args=(https_httpd,), daemon=True))

    for t in threads:
        t.start()
    try:
        threads[0].join()
    except KeyboardInterrupt:
        print("\nBye")
    finally:
        http_httpd.shutdown()
        if https_httpd is not None:
            https_httpd.shutdown()
