"""Hold one successful engine reply while a client test changes project inputs."""
import json
from pathlib import Path
import signal
import subprocess
import sys
import threading
import time

engine, method, ready_name, release_name, *arguments = sys.argv[1:]
ready, release = Path(ready_name), Path(release_name)
child = subprocess.Popen([engine, *arguments], stdin=subprocess.PIPE, stdout=subprocess.PIPE)
requests, lock = set(), threading.Lock()


def stop(signum, _frame):
    if child.poll() is None:
        child.terminate()
    raise SystemExit(128 + signum)


signal.signal(signal.SIGTERM, stop)


def pump(source, destination, outbound):
    while True:
        headers = []
        while True:
            line = source.readline()
            if not line:
                return
            headers.append(line)
            if line == b'\r\n':
                break
        size = next(int(line.split(b':', 1)[1]) for line in headers
                    if line.lower().startswith(b'content-length:'))
        body = source.read(size)
        if len(body) != size:
            raise EOFError('Incomplete protocol frame')
        message = json.loads(body)
        if not outbound and message.get('method') == method and 'id' in message:
            with lock:
                requests.add(message['id'])
        if outbound and ('result' in message or 'error' in message):
            with lock:
                selected = message.get('id') in requests
                requests.discard(message.get('id'))
            if selected and 'result' in message and not ready.exists():
                temporary = ready.with_suffix('.tmp')
                temporary.write_text(json.dumps(message))
                temporary.replace(ready)
                deadline = time.monotonic() + 20
                while not release.exists():
                    if time.monotonic() >= deadline:
                        raise TimeoutError('Test did not release the held reply')
                    time.sleep(0.01)
        destination.write(b''.join(headers) + body)
        destination.flush()


threading.Thread(target=pump, args=(sys.stdin.buffer, child.stdin, False), daemon=True).start()
try:
    pump(child.stdout, sys.stdout.buffer, True)
finally:
    if child.poll() is None:
        child.terminate()
    child.wait(timeout=5)
