# stdlib/net.py
import socket
import subprocess

def _test():
    print("hello world!")

# Каждая функция принимает (args: list, env: Env) → возвращает значение
def _ping(args, env):
    host = args[0]
    result = subprocess.run(
        ["ping", "-c", "1", "-W", "1", host],
        capture_output=True, text=True
    )
    return result.returncode == 0   # bool

def _port_open(args, env):
    host, port = args[0], int(args[1])
    try:
        s = socket.create_connection((host, port), timeout=2)
        s.close()
        return True
    except OSError:
        return False

def _resolve(args, env):
    try:
        return socket.gethostbyname(args[0])
    except socket.gaierror:
        return None

# Обязательно: словарь экспортов модуля
EXPORTS = {
    "ping":      _ping,
    "port_open": _port_open,
    "resolve":   _resolve,
    "test": _test
}