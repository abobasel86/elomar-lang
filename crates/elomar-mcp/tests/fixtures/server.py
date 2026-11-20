"""A tiny MCP server over stdio, for tests of the Elomar MCP client.

Usage: server.py MODE, where MODE is
  modern         protocol 2026-07-28 (no handshake, `_meta` on each request)
  legacy         needs `initialize`; rejects unknown requests before it
  silent-legacy  like legacy, but ignores `server/discover` entirely
  crash          exits right away with a message on stderr
"""

import json
import sys

MODE = sys.argv[1] if len(sys.argv) > 1 else "modern"
MODERN = "2026-07-28"

TOOLS = [
    {
        "name": "add",
        "description": "Adds two numbers",
        "inputSchema": {
            "type": "object",
            "properties": {"a": {"type": "number"}, "b": {"type": "number"}},
            "required": ["a", "b"],
        },
    },
    {
        "name": "fail",
        "description": "Always fails",
        "inputSchema": {"type": "object", "additionalProperties": False},
    },
    {
        "name": "ask",
        "description": "Needs user input",
        "inputSchema": {"type": "object"},
    },
]

if MODE == "crash":
    sys.stderr.write("cannot open the notes database\n")
    sys.exit(3)

initialized = False


def reply(msg_id, result=None, error=None):
    msg = {"jsonrpc": "2.0", "id": msg_id}
    if error is not None:
        msg["error"] = error
    else:
        msg["result"] = result
    sys.stdout.write(json.dumps(msg) + "\n")
    sys.stdout.flush()


def complete(result):
    if MODE == "modern":
        result["resultType"] = "complete"
    return result


for line in sys.stdin:
    msg = json.loads(line)
    method = msg.get("method")
    msg_id = msg.get("id")
    params = msg.get("params") or {}
    if msg_id is None:
        if method == "notifications/initialized":
            initialized = True
        continue
    if MODE == "modern":
        meta = params.get("_meta") or {}
        version = meta.get("io.modelcontextprotocol/protocolVersion")
        if version != MODERN or "io.modelcontextprotocol/clientCapabilities" not in meta:
            reply(msg_id, error={"code": -32602, "message": "missing _meta"})
            continue
        if method == "server/discover":
            reply(msg_id, complete({"supportedVersions": [MODERN], "capabilities": {"tools": {}},
                                    "ttlMs": 0, "cacheScope": "public"}))
            continue
    else:
        if method == "initialize":
            reply(msg_id, {"protocolVersion": params.get("protocolVersion"),
                           "capabilities": {"tools": {}},
                           "serverInfo": {"name": "fixture", "version": "1"}})
            # A legacy server may send its own requests; the client must answer.
            sys.stdout.write(json.dumps({"jsonrpc": "2.0", "id": "p1", "method": "ping"}) + "\n")
            sys.stdout.flush()
            continue
        if not initialized:
            if MODE == "silent-legacy":
                continue
            reply(msg_id, error={"code": -32601, "message": "Method not found"})
            continue
    if method == "tools/list":
        # Two pages, to test pagination.
        if params.get("cursor") == "page2":
            reply(msg_id, complete({"tools": TOOLS[1:]}))
        else:
            reply(msg_id, complete({"tools": TOOLS[:1], "nextCursor": "page2"}))
    elif method == "tools/call":
        name = params.get("name")
        args = params.get("arguments") or {}
        sys.stdout.write(json.dumps({"jsonrpc": "2.0", "method": "notifications/progress",
                                     "params": {"progress": 1}}) + "\n")
        if name == "add":
            total = args["a"] + args["b"]
            reply(msg_id, complete({"content": [{"type": "text", "text": str(total)}],
                                    "structuredContent": {"sum": total}}))
        elif name == "fail":
            reply(msg_id, complete({"content": [{"type": "text", "text": "it broke"}],
                                    "isError": True}))
        elif name == "ask":
            reply(msg_id, {"resultType": "input_required", "requestState": "x"})
        else:
            reply(msg_id, error={"code": -32602, "message": f"Unknown tool: {name}"})
    elif msg_id == "p1" or method is None:
        continue
    else:
        reply(msg_id, error={"code": -32601, "message": "Method not found"})
