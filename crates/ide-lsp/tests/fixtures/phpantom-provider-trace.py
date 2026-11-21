"""Record content-free protocol events for the owned provider-sync fixture."""
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import threading

engine, control_name, *arguments = sys.argv[1:]
control = Path(control_name)
root = str(Path.cwd().resolve())
child = subprocess.Popen([engine, *arguments], stdin=subprocess.PIPE, stdout=subprocess.PIPE)
record_lock = threading.Lock()
requests_lock = threading.Lock()
requests = {}


def record(event):
    encoded = (json.dumps(dict(event, root=root)) + '\n').encode('utf-8')
    with record_lock:
        fd = os.open(control / 'events.jsonl', os.O_WRONLY | os.O_CREAT | os.O_APPEND, 0o600)
        try:
            assert os.write(fd, encoded) == len(encoded)
        finally:
            os.close(fd)


record({'method': 'engineStarted', 'enginePid': child.pid})


def stop(signum, _frame):
    if child.poll() is None:
        child.terminate()
    raise SystemExit(128 + signum)


signal.signal(signal.SIGTERM, stop)


def pump(source, destination, incoming):
    try:
        while True:
            headers = []
            while True:
                line = source.readline()
                if not line:
                    return
                headers.append(line)
                if line == b'\r\n':
                    break
            size = next(int(line.split(b':', 1)[1]) for line in headers if line.lower().startswith(b'content-length:'))
            body = source.read(size)
            if len(body) != size:
                raise EOFError('Incomplete protocol frame')
            if incoming:
                message = json.loads(body)
                method = message.get('method')
                if method == 'phpWorkbench/jsonTranslationResource' and 'id' in message:
                    with requests_lock:
                        requests[message['id']] = message['params']
                if method in {'textDocument/didOpen', 'textDocument/didChange', 'textDocument/didClose', 'textDocument/hover'}:
                    document = message['params']['textDocument']
                    event = {'method': method, 'uri': document['uri'], 'version': document.get('version')}
                    if method == 'textDocument/didOpen':
                        event['sha256'] = hashlib.sha256(document['text'].encode('utf-8')).hexdigest()
                    record(event)
            else:
                message = json.loads(body)
                with requests_lock:
                    params = requests.pop(message.get('id'), None)
                if params is not None:
                    result = message.get('result') or {}
                    sources = result.get('providerSources') or {}
                    providers = [{key: value.get(key) for key in ['className', 'uri', 'sha256']}
                                 for value in sources.get('providers', [])]
                    record({'method': 'providerReceipt', 'projectUri': params['projectUri'],
                            'resourceUri': params['resourceUri'], 'rootUri': result.get('rootUri'),
                            'providers': providers, 'unresolved': sources.get('unresolved')})
            destination.write(b''.join(headers) + body)
            destination.flush()
    finally:
        if incoming:
            destination.close()


threading.Thread(target=pump, args=(sys.stdin.buffer, child.stdin, True), daemon=True).start()
try:
    pump(child.stdout, sys.stdout.buffer, False)
finally:
    if child.poll() is None:
        child.terminate()
    child.wait(timeout=5)
