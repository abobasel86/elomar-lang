# Elomar-Lang with a model on your computer

[Ollama](https://ollama.com) runs open models locally: no API key, no
cost per call, and your data stays on your computer.

1. Install Ollama and pull two models:

   ```bash
   ollama pull qwen2.5:7b
   ollama pull nomic-embed-text
   ```

2. Run:

   ```bash
   elomar run examples/ai-local/main.elo
   ```

Any model you have pulled works: change `model` in `elomar.toml`. The
program is the same as for OpenAI or Claude; only `elomar.toml` changes.
`[ai.cache]` keeps the vectors of `ai.search`, so the second run does not
embed the notes again.

---

## بالعربية

ثبّت [Ollama](https://ollama.com) ونزّل النموذجين بالأمرين أعلاه، ثم شغّل
`elomar run examples/ai-local/main.elo`. لا يلزم مفتاح ولا تُدفع كلفة لكل استدعاء، وتبقى بياناتك على جهازك.
ويمكنك استخدام أي نموذج نزّلته بتغيير `model` في `elomar.toml`.
