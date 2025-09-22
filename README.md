<!-- LOGO -->
<p align="center">
  <img src="https://github.com/user-attachments/assets/c3582cb8-c1cc-41ab-9ed1-f8fbde4d8c21" width="200" alt="fur logo"/>
</p>

<h1 align="center">FUR</h1>
<p align="center">
  <strong>Forkable, Unearthable, Recursive memory tracker</strong><br/>
Like git, but for conversations, ideas, and AI chats.
</p>

---

## 🤔 What is FUR?

Scrolling through endless chats to find that one reply is painful.  
FUR makes it easy to **track, branch, and preserve** your conversations as trees you can navigate, fork, and export.

With FUR you can:

* **Jot** quick notes or attach Markdown files.
* **Chat** long-form messages interactively (paste documents straight in).
* **Branch & fork** conversations into multiple futures.
* **Jump** backward or forward to any message.
* **See** threads as timelines or trees.
* **Switch** between multiple threads easily.
* **Assign avatars** (🦊 you, 🤖 bots, 👤 others — customizable).
* **Script conversations with `.frs` files**.
* **Export** threads to Markdown or PDF.

Think of it as a **version control system for your thoughts**.

---

## 🌟 What’s New in v0.3.5 — Enter the Chat Den 🦊💬

* **`fur chat`** → interactive, long-form jotting.  
  Paste Markdown, essays, or multi-line rants directly into the CLI.  
  By default, FUR suggests saving inside a `chats/` folder.  

* **Better ergonomics** → `chat` is the natural sibling of `jot`.  
  - `fur jot` → quick scratches.  
  - `fur chat` → longer tales.  

* **Tests** → new `tests/chat.rs` covers file + message creation.  

---

## 🛠 How it works

FUR keeps everything inside a local `.fur/` folder:

* `.fur/index.json` → global state  
* `.fur/threads/*.json` → one per thread  
* `.fur/messages/*.json` → one per message  
* `.fur/avatars.json` → avatar mappings  

### Example commands

```bash
# Start fresh
fur new "Penguin talks"

# Manage avatars
fur avatar andrew               # set yourself (🦊 main)
fur avatar tengu --emoji 👺     # create a custom avatar with emoji
fur avatar --view

# Quick jot
fur jot "Just finished reading about quantum time crystals."

# Jot as a custom avatar
fur jot dr-strange "We’re in the endgame now."

# Long-form interactive jot (paste Markdown or docs)
fur chat gpt5

# Attach an existing markdown file
fur jot ai-helper --file examples/chats/QUANTUM_MANIFESTO.md

# Work with scripts
fur run examples/quantum_playground.frs
# or just:
fur examples/quantum_playground.frs

# Export views
fur timeline --contents --out CONVO.md
fur timeline --contents --out convo.pdf



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

Avatars keep things clear: 🦊 (you), 🤖 (AI/bots), 👤 (others). But you can always customize them (`fur avatar tengu --emoji 👺`).

**Goal:** Make recursive thinking natural.

---

## 🛣 Roadmap to v1.0

✅ **Already complete (v0.3)**

* Threads (`fur new`, `fur thread`)
* Avatars (`fur avatar`)
* Jotting text & files (`fur jot`)
* Tree / Timeline views
* Jumping & forking
* `.frs` scripting system (branching supported here)
* VS Code highlighting for `.frs`
* Thread import / export
* Markdown & PDF rendering
* Polished exports (Markdown/PDF with styles, embedded assets)

🔜 **Planned for future releases**

* `fur rm` → delete messages directly in the CLI
* `fur move` → replace / reorder messages in a thread
* `fur branch` → create branches interactively in the CLI (currently only in `.frs` scripts)
* Interactive editing flows for power users
* Richer exports (metadata, tags, avatars)

🎉 **v1.0 Milestone**

* Full editing suite: add, delete, move, replace, fork, branch — all stable
* Rock-solid `.frs` import/export parity (round-trip safe)
* Robust test coverage & docs
* Consider cross-platform packaging (Homebrew, Scoop, etc.)

---

## 📜 License

MIT, like almost everything else that's friendly and open-source.

