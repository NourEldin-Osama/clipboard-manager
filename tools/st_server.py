#!/usr/bin/env python3
"""Serve Sentence Transformers models over an OpenAI-compatible embeddings API
for clipboard-manager (or anything else that speaks /v1/embeddings).

    pip install -U sentence-transformers pillow
    python st_server.py --model google/embeddinggemma-300m --model sentence-transformers/all-MiniLM-L6-v2

Then in clipboard-manager: Settings -> Embedding model -> Local server ->
"Sentence Transformers" preset (http://127.0.0.1:8001/v1) and pick a model.

Endpoints
  GET  /health
  GET  /v1/models
  POST /v1/embeddings   {"model": "...", "input": "text" | ["text", ...]}
                        {"model": "...", "messages": [{"role": "user", "content":
                            [{"type": "image_url", "image_url": {"url": "data:image/png;base64,..."}}]}]}

The optional "X-Embedding-Role: query|document" header selects the model's own
query/document prompt when it defines one (e.g. EmbeddingGemma).
Multi-vector models (ColBERT/ColPali style) return one vector per token.
Listens on 127.0.0.1 only by default; nothing is sent anywhere. It answers
only requests addressed to it, and refuses requests from web pages and
requests over 64 MB.
"""

import argparse
import base64
import hmac
import io
import json
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

import numpy as np

MODELS = {}
LOCK = threading.Lock()
ARGS = None
# Room for several images sent as base64.
MAX_BODY_BYTES = 64 * 1024 * 1024
LOOPBACK = ("127.0.0.1", "localhost", "::1")
# The Host headers a client on this PC sends; set in main() from the port.
ALLOWED_HOSTS = set()


def load(name):
    if name not in MODELS:
        from sentence_transformers import SentenceTransformer

        kwargs = {"trust_remote_code": ARGS.trust_remote_code}
        if ARGS.device:
            kwargs["device"] = ARGS.device
        print(f"loading {name} ...", flush=True)
        MODELS[name] = SentenceTransformer(name, **kwargs)
    return MODELS[name]


def pick_model(requested):
    names = ARGS.model
    if requested in names or requested in MODELS:
        return requested
    if not requested:
        return names[0]
    raise KeyError(f"model '{requested}' is not served; available: {', '.join(names)}")


def images_from_messages(messages):
    from PIL import Image

    out = []
    for m in messages or []:
        content = m.get("content")
        if not isinstance(content, list):
            continue
        for part in content:
            if part.get("type") != "image_url":
                continue
            url = part["image_url"]["url"]
            if not url.startswith("data:"):
                raise ValueError("only data: URIs are supported for images")
            raw = base64.b64decode(url.split(",", 1)[1])
            out.append(Image.open(io.BytesIO(raw)).convert("RGB"))
    return out


def to_json_vector(v):
    if hasattr(v, "detach"):  # torch tensor
        v = v.detach().float().cpu().numpy()
    return np.asarray(v, dtype=np.float32).tolist()


def encode(model, inputs, role):
    kwargs = {"convert_to_numpy": False, "normalize_embeddings": False}
    prompts = getattr(model, "prompts", None) or {}
    if role in prompts and all(isinstance(x, str) for x in inputs):
        kwargs["prompt_name"] = role
    out = model.encode(inputs, **kwargs)
    return [to_json_vector(e) for e in out]


class Handler(BaseHTTPRequestHandler):
    def _send(self, code, payload):
        body = json.dumps(payload).encode()
        self.send_response(code)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, fmt, *args):
        if ARGS.verbose:
            super().log_message(fmt, *args)

    def _allowed(self):
        # A web page can reach this port through the browser: by a plain
        # form post, or by DNS rebinding (its own host name pointed at
        # 127.0.0.1). Browsers send an Origin header and the page's host
        # name; the app sends neither.
        if self.headers.get("Origin") is not None:
            self._send(403, {"error": {"message": "requests from web pages are not accepted"}})
            return False
        host = self.headers.get("Host", "").lower()
        if ARGS.host in LOOPBACK and host not in ALLOWED_HOSTS:
            self._send(403, {"error": {"message": f"unexpected Host header: {host}"}})
            return False
        return True

    def _authorized(self):
        if not ARGS.api_key:
            return True
        sent = self.headers.get("Authorization", "").encode()
        if hmac.compare_digest(sent, f"Bearer {ARGS.api_key}".encode()):
            return True
        self._send(401, {"error": {"message": "invalid or missing API key"}})
        return False

    def do_GET(self):
        if not self._allowed():
            return
        path = self.path.rstrip("/")
        if path in ("/health", "/v1/health"):
            return self._send(200, {"status": "ok"})
        if path in ("/v1/models", "/models"):
            if not self._authorized():
                return
            return self._send(200, {"object": "list", "data": [{"id": n, "object": "model", "owned_by": "local"} for n in ARGS.model]})
        self._send(404, {"error": {"message": "not found"}})

    def do_POST(self):
        if not self._allowed():
            return
        if self.path.rstrip("/") not in ("/v1/embeddings", "/embeddings"):
            return self._send(404, {"error": {"message": "not found"}})
        if not self._authorized():
            return
        try:
            length = int(self.headers.get("Content-Length", 0))
        except ValueError:
            return self._send(400, {"error": {"message": "invalid Content-Length"}})
        if length > MAX_BODY_BYTES:
            return self._send(413, {"error": {"message": f"requests are limited to {MAX_BODY_BYTES} bytes"}})
        try:
            req = json.loads(self.rfile.read(length) or b"{}")
            name = pick_model(req.get("model", ""))
            role = (self.headers.get("X-Embedding-Role") or "").lower()
            if "messages" in req:
                inputs = images_from_messages(req["messages"])
                if not inputs:
                    raise ValueError("no image found in messages")
            else:
                inputs = req.get("input", [])
                if isinstance(inputs, str):
                    inputs = [inputs]
            with LOCK:
                vectors = encode(load(name), inputs, role)
            data = [{"object": "embedding", "index": i, "embedding": v} for i, v in enumerate(vectors)]
            self._send(200, {"object": "list", "model": name, "data": data, "usage": {"prompt_tokens": 0, "total_tokens": 0}})
        except KeyError as e:
            self._send(404, {"error": {"message": str(e)}})
        except Exception as e:  # report every failure to the client
            self._send(400, {"error": {"message": f"{type(e).__name__}: {e}"}})


def main():
    global ARGS
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--model", action="append", required=True, help="Hugging Face id or local path; repeat to serve several")
    p.add_argument("--host", default="127.0.0.1")
    p.add_argument("--port", type=int, default=8001)
    p.add_argument("--device", default=None, help="cpu, cuda, mps ... (default: auto)")
    p.add_argument("--trust-remote-code", action="store_true", help="needed for e.g. BiUltraVBERT / ColUltraVBERT")
    p.add_argument("--api-key", default=None, help="require 'Authorization: Bearer <key>' on API calls")
    p.add_argument("--preload", action="store_true", help="load all models at start-up instead of on first use")
    p.add_argument("--verbose", action="store_true")
    ARGS = p.parse_args()
    for name in ("127.0.0.1", "localhost", "[::1]"):
        ALLOWED_HOSTS.add(f"{name}:{ARGS.port}")
        if ARGS.port == 80:
            ALLOWED_HOSTS.add(name)
    if ARGS.host not in LOOPBACK and not ARGS.api_key:
        print("warning: listening beyond this PC without --api-key: anyone who can reach it can use it", flush=True)
    if ARGS.preload:
        for n in ARGS.model:
            load(n)
    print(f"serving {', '.join(ARGS.model)} on http://{ARGS.host}:{ARGS.port}/v1", flush=True)
    ThreadingHTTPServer((ARGS.host, ARGS.port), Handler).serve_forever()


if __name__ == "__main__":
    main()
