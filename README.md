<div align="center">

# digi Grok Build (`dgrok`)

**digi Grok Build** is a branded fork of SpaceXAI’s open-source [Grok Build](https://github.com/xai-org/grok-build).  
**Code base = upstream xAI.** digi only changes product name / install surface (`dgrok`).

[Installing](#installing) ·
[Custom models (upstream)](#custom-models-any-provider) ·
[Documentation](#documentation) ·
[Development](#development)

</div>

---

## What digi is (and is not)

| | |
|--|--|
| **Is** | xAI Grok Build runtime + **digi branding** (`dgrok`, DigiGoon install) |
| **Is not** | A reimplementation of multi-provider APIs — that is **already in upstream** |
| **Policy** | Rebase/reset onto `xai-org/grok-build` regularly; keep digi diff **minimal** |

---

## Installing

Same one-liner shape as official Grok; digi uses **GitHub Releases** (this repo), not x.ai CDN.

```sh
curl -fsSL https://raw.githubusercontent.com/DigiGoon/digi-grok-build/main/scripts/install.sh | bash
dgrok --version
```

Restart the terminal if the installer reports that another `dgrok` appears earlier in `PATH`.

```powershell
irm https://raw.githubusercontent.com/DigiGoon/digi-grok-build/main/scripts/install.ps1 | iex
dgrok --version
```

If no release asset exists yet, the script **falls back to building from source**.  
Dev: `DGROK_FROM_SOURCE=1 bash scripts/install.sh` or `./scripts/install-from-source.sh --release`.

Prebuilt assets (when published): `dgrok-linux-x86_64`, `dgrok-darwin-*`, `dgrok-windows-x86_64.exe`.

---

## Custom models (any provider)

Stock Grok Build already supports multi-provider endpoints via **`~/.grok/config.toml`**.  
Full reference: [`11-custom-models.md`](crates/codegen/xai-grok-pager/docs/user-guide/11-custom-models.md).

digi adds a **thin helper** that only reads/writes that same official file (no parallel provider stack):

```sh
dgrok doctor                          # config path, [model.*], env keys set?
dgrok provider presets                # nvidia / openai / anthropic / ollama / openrouter
dgrok provider add nvidia --model z-ai/glm-5.2 --set-default
export NVIDIA_API_KEY=…               # secrets stay in the environment
dgrok -m nvidia
# TUI: /provider list | presets | add …
```

Or edit TOML by hand:

```toml
# ~/.grok/config.toml
[model.claude-opus]
model = "claude-opus-4-6"
base_url = "https://api.anthropic.com/v1"
api_backend = "messages"
env_key = "ANTHROPIC_API_KEY"
```

```sh
dgrok models
dgrok -p "Hello" -m claude-opus
```

---

## Documentation

| Doc | Purpose |
|-----|---------|
| [User guide](crates/codegen/xai-grok-pager/docs/user-guide/) | Full Grok Build product docs (upstream) |
| [Custom models](crates/codegen/xai-grok-pager/docs/user-guide/11-custom-models.md) | Multi-provider config |
| Upstream overview | https://docs.x.ai/build/overview |

---

## Building from source

Requirements: Rust (`rust-toolchain.toml`), protoc (see repo `bin/protoc` / PATH).

```sh
cargo build -p xai-grok-pager-bin --release
./target/release/dgrok --version
```

---

## License

Same as upstream Grok Build (see repository LICENSE).  
Upstream: [xai-org/grok-build](https://github.com/xai-org/grok-build). digi branding: DigiGoon.
