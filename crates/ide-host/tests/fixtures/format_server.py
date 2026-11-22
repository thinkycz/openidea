import json
import pathlib
import sys
import time

control = pathlib.Path(sys.argv[1])
documents = {}


def send(message):
    body = json.dumps(message).encode()
    sys.stdout.buffer.write(f"Content-Length: {len(body)}\r\n\r\n".encode() + body)
    sys.stdout.buffer.flush()


while True:
    headers = {}
    while True:
        line = sys.stdin.buffer.readline()
        if not line:
            sys.exit(0)
        if line == b"\r\n":
            break
        key, value = line.decode().split(":", 1)
        headers[key.lower()] = value.strip()
    message = json.loads(sys.stdin.buffer.read(int(headers["content-length"])))
    method = message.get("method")
    params = message.get("params", {})
    if method == "initialize":
        result = {"capabilities": {"textDocumentSync": 1, "documentFormattingProvider": True, "positionEncoding": "utf-16"}}
    elif method == "textDocument/didOpen":
        document = params["textDocument"]
        documents[document["uri"]] = document["text"]
        continue
    elif method == "textDocument/didChange":
        documents[params["textDocument"]["uri"]] = params["contentChanges"][-1]["text"]
        continue
    elif method == "textDocument/formatting":
        text = documents[params["textDocument"]["uri"]]
        row = text.count("\n")
        column = len(text.rsplit("\n", 1)[-1].encode("utf-16-le")) // 2
        (control / "ready").write_text(json.dumps({"uri": params["textDocument"]["uri"], "text": text}))
        deadline = time.monotonic() + 20
        while not (control / "release").exists():
            if time.monotonic() > deadline:
                sys.exit(2)
            time.sleep(0.01)
        result = [{"range": {"start": {"line": 0, "character": 0}, "end": {"line": row, "character": column}}, "newText": text.replace("  echo", "    echo")}]
    elif method == "shutdown":
        result = None
    elif method == "exit":
        sys.exit(0)
    else:
        if "id" not in message:
            continue
        result = None
    if "id" in message:
        send({"jsonrpc": "2.0", "id": message["id"], "result": result})
