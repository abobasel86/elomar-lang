# Elomar-Lang with Claude

1. Put your Anthropic API key in the environment (never in a file):

   ```bash
   export ANTHROPIC_API_KEY="sk-ant-..."
   ```

   On Windows PowerShell: `$env:ANTHROPIC_API_KEY = "sk-ant-..."`.
2. Run:

   ```bash
   elomar run examples/ai-claude/main.elo
   ```

`elomar.toml` uses `claude-opus-5-5`. Change `model` to use another Claude
model, for example `claude-haiku-5-5` for a cheaper one.

The same model works with `ai.run` (your functions as tools) and
`agent.run` (tools of MCP servers): set `provider = "anthropic"` in
[`examples/ai-tools`](../ai-tools) or [`examples/agents`](../agents).

The safeguards are the same as for OpenAI (see
[`examples/ai-openai`](../ai-openai/README.md)): the `net` effect, the
`network_hosts` allowlist, the key read at call time and masked in errors, a
call budget, answers checked against `Ticket`, and your input sent as data.
If Claude declines a request, the answer is `AIError.PolicyDenied`.

---

## بالعربية

ضع مفتاح Anthropic في متغير البيئة `ANTHROPIC_API_KEY` (لا تكتبه في أي ملف)، ثم شغّل
`elomar run examples/ai-claude/main.elo`. النموذج الافتراضي `claude-opus-5-5`، ويمكن تغييره
من `model` في `elomar.toml`.
