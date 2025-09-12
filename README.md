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
* **Branch & fork** conversations into multiple futures.
* **Jump** backward or forward to any message.
* **See** threads as timelines or trees.
* **Switch** between multiple threads easily.
* **Assign avatars** (🦊 you, 🤖 bots, 👤 others — customizable).
* **Script conversations with `.frs` files**.
* **Export** threads to Markdown or PDF.

Think of it as a **version control system for your thoughts**.

---

## 🌟 What’s New in v0.3.0

**Highlights**

* **`.frs` scripting** → Write branching chats declaratively and load them into FUR.
* **Import / export** → Save any thread back into `.frs` or load existing ones.
* **Rich exports** → Export timelines to Markdown or PDF, with branches preserved.
* **FurScript syntax highlighting** → Official [VS Code extension](https://marketplace.visualstudio.com/items?itemName=andrewgarcia.fur-frs) for `.frs` files.

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

# Jot a message as yourself (🦊)
fur jot "Just finished reading about quantum time crystals."

# Jot a message as a custom avatar (👤 or 🤖 depending on name/emoji)
fur jot dr-strange "We’re in the endgame now."

# Attach a markdown file
fur jot ai-helper --file examples/chats/QUANTUM_MANIFESTO.md

# Provide both text and file (text will show in timeline, file in exports)
fur jot ai-helper "Here’s the updated draft." --file examples/chats/QUANTUM_MANIFESTO.md

# Longform main user entry (text + equations doc)
fur jot --text "Nonlocality still breaks my brain." --markdown examples/chats/ENTANGLEMENT_EQS.md

# Work with scripts
fur load examples/quantum_playground.frs
fur save --out meeting_notes.frs

# Export views
fur timeline --contents --out convo.md
fur timeline --contents --out convo.pdf
```

---

## 📂 Examples

The repo ships with an `examples/` directory full of ready-to-run `.frs` scripts and linked Markdown files.
These make it easy to try FUR without writing anything from scratch.

```bash
# Quantum demo with multiple AI avatars + branching
fur load examples/quantum_playground.frs
fur timeline --contents

# Outer-wordly penguin encounter
fur load examples/penguin_verses.frs
fur tree

# Practical meeting notes - with attached report
fur load examples/department_meeting.frs
fur timeline --out meeting.pdf
```

Contents include:

* **Quantum Playground** → cinematic physics, math, and startup pitch branches
* **Penguin Verses** → recursive alien penguin gospel, with linked docs like *Meeponomicon*
* **Department Meeting** → structured business chat w/ attached summary report
* **Dad Jokes** → humor tree with branching punchlines
* Plus extra flavor texts in `examples/chats/` (manifestos, equations, poems, absurdities)

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

