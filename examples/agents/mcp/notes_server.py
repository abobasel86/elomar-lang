"""A small MCP server with a list of notes, kept in memory.

It speaks MCP over stdio: one JSON-RPC message per line. It answers both
the current protocol (2026-07-28, where every request carries `_meta` and
there is no handshake) and older clients that send `initialize` first.
Elomar starts it because elomar.toml lists it under [mcp.servers.notes].
"""

import json
import sys

notes = []

TOOLS = [
    {
        "name": "add_note",
        "description": "Adds a note to the list",
        "inputSchema": {
            "type": "object",
            "properties": {"text": {"type": "string", "description": "The note"}},
            "required": ["text"],
        },
    },
    {
        "name": "count_notes",
        "description": "How many notes there are",
        "inputSchema": {"type": "object", "additionalProperties": False},
    },
    {
        "name": "delete_all",
        "description": "Deletes every note",
        "inputSchema": {"type": "object", "additionalProperties": False},
    },
]


def text(t, error=False):
    return {"resultType": "complete", "content": [{"type": "text", "text": t}], "isError": error}


def call(name, args):
    if name == "add_note":
        notes.append(args.get("text", ""))
        return text(f"added note {len(notes)}")
    if name == "count_notes":
        return text(str(len(notes)))
    if name == "delete_all":
        notes.clear()
        return text("deleted")
    return None


def handle(method, params):
    if method == "server/discover":
        return {"resultType": "complete", "supportedVersions": ["2026-07-28", "2025-11-25"],
                "capabilities": {"tools": {}}, "ttlMs": 0, "cacheScope": "public"}
    if method == "initialize":
        return {"protocolVersion": "2025-11-25", "capabilities": {"tools": {}},
                "serverInfo": {"name": "notes", "version": "1.0"}}
    if method == "tools/list":
        return {"resultType": "complete", "tools": TOOLS}
    if method == "tools/call":
        return call(params.get("name"), params.get("arguments") or {})
    return None


for line in sys.stdin:
    msg = json.loads(line)
    if "id" not in msg or "method" not in msg:
        continue  # notifications and answers need no reply
    result = handle(msg["method"], msg.get("params") or {})
    if result is None:
        reply = {"jsonrpc": "2.0", "id": msg["id"],
                 "error": {"code": -32601, "message": f"unknown: {msg['method']}"}}
    else:
        reply = {"jsonrpc": "2.0", "id": msg["id"], "result": result}
    sys.stdout.write(json.dumps(reply) + "\n")
    sys.stdout.flush()
