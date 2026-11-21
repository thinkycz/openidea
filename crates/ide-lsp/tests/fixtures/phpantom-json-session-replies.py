"""Hold selected real replies without blocking other protocol frames."""
import json
from pathlib import Path
import signal
import subprocess
import sys
import threading
import time

engine, control_name, *arguments = sys.argv[1:]
control = Path(control_name)
methods = set(json.loads((control / 'methods.json').read_text()))
child = subprocess.Popen([engine, *arguments], stdin=subprocess.PIPE, stdout=subprocess.PIPE)
requests, held = {}, set()
state_lock, output_lock, events_lock = threading.Lock(), threading.Lock(), threading.Lock()


def stop(signum, _frame):
    if child.poll() is None:
        child.terminate()
    raise SystemExit(128 + signum)


signal.signal(signal.SIGTERM, stop)


def emit(frame):
    with output_lock:
        sys.stdout.buffer.write(frame)
        sys.stdout.buffer.flush()


def release_reply(method, frame):
    release = control / (method.rsplit('/', 1)[1] + '.release')
    deadline = time.monotonic() + 20
    while not release.exists():
        if time.monotonic() >= deadline:
            raise TimeoutError('Test did not release the real reply')
        time.sleep(0.01)
    try:
        emit(frame)
    except BrokenPipeError:
        pass


def pump(source, outbound):
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
        frame = b''.join(headers) + body
        if not outbound:
            method = message.get('method')
            if method in methods and 'id' in message:
                with state_lock:
                    requests[message['id']] = method
            child.stdin.write(frame)
            child.stdin.flush()
            events = []
            if method in {'textDocument/didOpen', 'textDocument/didChange', 'textDocument/didClose'}:
                document = message['params']['textDocument']
                events.append({'method': method, 'uri': document['uri'], 'version': document.get('version')})
            elif method == 'workspace/didChangeWatchedFiles':
                events.extend({'method': method, 'uri': change['uri']} for change in message['params']['changes'])
            if events:
                with events_lock, (control / 'events.jsonl').open('a') as log:
                    for event in events:
                        log.write(json.dumps(event) + '\n')
            continue
        with state_lock:
            method = requests.pop(message.get('id'), None) if ('result' in message or 'error' in message) else None
            selected = method is not None and method not in held and 'result' in message
            if selected:
                held.add(method)
        if selected:
            ready = control / (method.rsplit('/', 1)[1] + '.json')
            temporary = ready.with_suffix('.tmp')
            temporary.write_text(json.dumps(message))
            temporary.replace(ready)
            threading.Thread(target=release_reply, args=(method, frame), daemon=True).start()
        else:
            emit(frame)


threading.Thread(target=pump, args=(sys.stdin.buffer, False), daemon=True).start()
try:
    pump(child.stdout, True)
finally:
    if child.poll() is None:
        child.terminate()
    child.wait(timeout=5)
