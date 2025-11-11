<!-- LOGO -->
<p align="center">
  <img src="https://github.com/user-attachments/assets/c3582cb8-c1cc-41ab-9ed1-f8fbde4d8c21" width="200" alt="fur logo"/>
</p>

<h1 align="center">FUR</h1>
<p align="center">
  <strong>Forkable, Unearthable, Recursive memory tracker</strong><br/>
Like git, but for conversations, ideas, and AI chats.
</p>

<p align="center">
  👉 For a more visual overview, see the <a href="https://furchats.vercel.app/">FURChats portfolio site</a>.
</p>

---

## 🤔 What is FUR?

Scrolling through endless chats to find that one reply is painful.  
FUR makes it easy to **track, branch, and preserve** your conversations as trees you can navigate, fork, and export.

With FUR you can:

* **Jot** quick notes or attach Markdown files.
* **Chat** long-form messages interactively (paste documents straight in).
* **Branch & fork** conversations into multiple timelines.
* **Jump** backward or forward to any message.
* **See** threads as timelines or trees.
* **Switch** between multiple threads easily.
* **Assign avatars** (🦊 you, 🤖 bots, 👤 others — customizable).
* **Script conversations with `.frs` files**.
* **Export** threads to Markdown or PDF.
* **(New)** Use **Git passthrough commands** directly through FUR.
* **(New)** View **Git status** integrated into `fur status` when inside a Git repo.

Think of it as a **version control system for your thoughts** — with optional Git integration when you want it.

---

## 🌟 What’s New in v0.4.1 — Git-Powered Status & Passthrough ⚡

### ✅ Git-enhanced `fur status`
If the project is inside a Git repository, `fur status` now:

* Shows your active FUR thread *and*  
* Shows `git status` output (fully colorized and real Git output)

No changes if there’s no `.git` — FUR stays minimal.

---

### ✅ Git passthrough commands (optional, zero-config)

FUR now exposes thin wrappers for common Git commands:

```bash
fur add <args...>
fur commit <args...>
fur push <args...>
fur pull <args...>
```

Everything after the subcommand is forwarded **raw** to Git:

* `fur commit -m "message"`
* `fur commit --amend`
* `fur add .`
* `fur push --force-with-lease`

Flags, hyphens, editor opens — everything works exactly like native git.

If no `.git` repo is found:

```
⚠️  No Git repository detected in this project.
```

Git passthrough is strictly **opt-in**.
FUR does not modify `.gitignore` and never writes to Git config.

---

## 🛠 How it works

FUR keeps everything inside a local `.fur/` folder:

* `.fur/index.json` → global state
* `.fur/threads/*.json` → one file per thread
* `.fur/messages/*.json` → one file per message
* `.fur/avatars.json` → avatar mappings

This folder is **local**, simple, and human-readable.

### Example commands

```bash
# Start fresh
fur new "Penguin talks"

# Manage avatars
fur avatar andrew            # set yourself (🦊 main)
fur avatar tengu --emoji 👺  # create custom avatar
fur avatar --view

# Quick jot
fur jot "Just finished reading about quantum time crystals."

# Jot with a custom avatar
fur jot dr-strange "We’re in the endgame now."

# Paste long text or Markdown interactively
fur chat gpt5

# Attach an existing Markdown file
fur jot ai-helper --file examples/chats/QUANTUM_MANIFESTO.md

# Run an .frs script
fur run examples/quantum_playground.frs

# Export views
fur timeline --contents --out CONVO.md
fur timeline --contents --out convo.pdf

# Git passthrough (if inside a Git repo)
fur add .
fur commit -m "message"
fur push
fur status   # shows git status too
```

---

## 🚀 Installation

### From crates.io

```bash
cargo install fur-cli
```

### From source

```bash
cargo install --path . --force
```

---

## 🐾 Philosophy

FUR is **minimal**.
It's not an AI client. It's a **memory tracker** that respects:

* Your brain's tendency to branch.
* Your need to retrace steps.
* Your desire to keep *everything*.

Git integration is optional — FUR uses it only when present, and gets out of the way when not.

**Goal:** Make recursive thinking natural.

---

## 🛣 Roadmap to v1.0

✅ **Already complete (v0.4)**

* Threads (`fur new`, `fur thread`)
* Avatars (`fur avatar`)
* Jot & Chat (`fur jot`, `fur chat`)
* Trees & Timelines
* Forking & jumping
* `.frs` scripting system
* Thread import/export
* PDF & Markdown exports
* Git passthrough (add/commit/push/pull)
* Git-enhanced `fur status`

🔜 **Planned for future releases**

* `fur rm` → delete messages in the CLI
* `fur move` → reorder/replace messages
* `fur branch` → interactive branch creation
* More editing tools
* Richer exports (metadata, tags, avatars)
* Optional helper: `fur gitignore`

🎉 **v1.0 Milestone**

* Full editing suite
* Round-trip-safe `.frs` import/export
* Robust test coverage
* Consider cross-platform packaging (Homebrew, Scoop, etc.)

---

## 📜 License

MIT, like almost everything else that's friendly and open-source.
