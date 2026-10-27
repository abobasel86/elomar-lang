# Elomar-Lang with OpenAI

1. Replace `REPLACE_WITH_SUPPORTED_MODEL_ID` in `elomar.toml` with an OpenAI
   model that supports structured outputs.
2. Put your API key in the environment (never in a file):

   ```bash
   export OPENAI_API_KEY="sk-..."
   ```

   On Windows PowerShell: `$env:OPENAI_API_KEY = "sk-..."`.
3. Run:

   ```bash
   cargo run -p elomar-cli -- run examples/ai-openai/main.elo
   ```

What protects you:

| Safeguard | Where |
|---|---|
| `net` must be declared on public functions that call a cloud model | compiler (`E3001`/`E3002`) |
| Only hosts in `[permissions] network_hosts` are contacted; redirects are not followed | runtime (`PolicyDenied`) |
| The key is read from `api_key_env` at call time and masked in error messages | provider |
| At most `max_calls_per_run` requests per run | runtime (`BudgetExceeded`) |
| Answers that do not match `Ticket` are rejected | runtime (`InvalidStructuredOutput`) |
| Your input is sent as data; the model is told not to follow instructions inside it | provider (system message) |

This reduces but does not eliminate prompt-injection risk; never give a model's
answer the power to take actions by itself.
