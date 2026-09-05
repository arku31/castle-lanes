# Knowledge Base

Operational knowledge for building and running Castle Lanes — the "how it
actually works" notes that live outside the code. Start here; each doc links
to the authoritative source it summarizes.

| Doc | What it covers |
|---|---|
| [Image generation via Codex](image-generation.md) | Generating new sprite/atlases with `codex exec` + the chroma-key/slice pipeline (no API key) |
| [Screenshot capture](screenshot-capture.md) | Taking game screenshots from the build shell, and the MCP helper restart caveat |
| [Architecture map](architecture.md) | Where the authoritative sim, netcode, fog, and client rendering live |
| [Balance workflow](balance-workflow.md) | Bot archetypes, metrics, and how to regenerate `docs/balance-report.md` |
| [Procedural audio](audio.md) | Regenerating all SFX with `tools/gen_sfx.py` |

Older, still-authoritative docs: [assets.md](../assets.md) (original art
pipeline), [counter-chains.md](counter-chains.md) (design contract),
[balance-report.md](balance-report.md) (latest harness output),
[bandwidth.md](bandwidth.md) (wire format measurements).
