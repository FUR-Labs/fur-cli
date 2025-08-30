<!-- LOGO -->
<p align="center">
  <img src="https://github.com/user-attachments/assets/c3582cb8-c1cc-41ab-9ed1-f8fbde4d8c21" width="200" alt="fur logo"/>
</p>

<h1 align="center">FUR</h1>
<p align="center">
  <strong>Forkable, Unearthable, Recursive memory tracker</strong><br/>
A memory tracker for your conversations, ideas, and AI chats.
</p>



---

## 🤔 What is FUR?

FUR is a tiny command-line tool that helps you **save and organize your chat messages** — especially your conversations with ChatGPT and other AIs — in a way that makes sense when things get complex.

Unlike note-taking apps, FUR lets your conversations **branch** into trees. You can:

- **Jot** text or link Markdown files as messages.  
- **Fork** conversations into multiple possible futures.  
- **Jump** backward or forward across messages.  
- **See** threads as a timeline or tree.  
- **Switch** between multiple threads.  
- **Assign avatars** (🦊 main, 👹 others) to track who said what.  

It's a **version control system for your thoughts** — like `git`, but for conversations.

> 🧠 _Think of FUR as “save points” for your mind. Every fork, every idea path, preserved forever._

---

## 🌟 Why would I want this?

- ✍️ **Writers**: Explore branching plotlines.  
- 🧑‍💻 **Developers**: Track coding convos with AI.  
- 🧠 **Students**: Study with side questions.  
- 📚 **Researchers**: Organize responses and notes.  
- 🤯 **Overthinkers**: Save *every possible what-if*.  

---

## 🛠 How does it work?

All data lives in a `.fur/` folder:  

- `.fur/index.json` → global state  
- `.fur/threads/*.json` → one per thread  
- `.fur/messages/*.json` → individual messages  
- `.fur/avatars.json` → avatar mappings  

### Example commands

```bash
# Start a new thread
fur new "Penguin talks"

# Add messages
fur jot --text "Penguins are weird birds."
fur jot jeff --text "Yo"        # from another avatar

# Manage avatars
fur avatar andrew                         # sets 🦊 main avatar
fur avatar --other ai --emoji 👹          # adds another avatar
fur avatar --view                         # list all avatars

# Manage threads
fur thread --view                         # list threads
fur thread d1f032d3                       # switch active thread

# Navigate inside a thread
fur tree                                  # tree view
fur timeline --verbose                    # linear view
fur jump --past 1                         # go back
fur status                                # current state
```

---

## 🚀 Installation

You'll need [Rust](https://www.rust-lang.org/tools/install).

```bash
cargo install --path .
```

Then ensure `~/.cargo/bin` is in your `PATH`.

Upgrade after edits:

```bash
cargo install --path . --force
```

---

## 🐾 Philosophy

FUR is **minimal**. It's not an AI client. It's a **memory tracker** that respects:

* Your brain's tendency to branch.
* Your need to retrace steps.
* Your desire to keep *everything*.

Goal: Make recursive thinking natural.

---

## 🛣 Roadmap (v0.2 → v1.0)

* ✅ Threads (`fur new`, `fur thread --view`, `fur thread <id>`)
* ✅ Avatars (`fur avatar`, `fur avatar --view`)
* ✅ Jotting text & files (`fur jot`)
* ✅ Tree / Timeline views
* ✅ Jumping & forking
* ⏳ Thread export / import
* ⏳ DSL for batch imports
* ⏳ Rich markdown rendering

---

## 📜 License

MIT, like almost everything else that's friendly and open-source.

