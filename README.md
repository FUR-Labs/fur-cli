<!-- LOGO -->
<p align="center">
  <img src="https://github.com/user-attachments/assets/c3582cb8-c1cc-41ab-9ed1-f8fbde4d8c21" width="200" alt="fur logo"/>
</p>

<h1 align="center">FUR 🐾</h1>
<p align="center">
  <strong>Forkable, Unearthable, Recursive memory tracker</strong><br/>
  A command-line tool for threading your thoughts like a raccoon hoards shiny things.
</p>

---

## ✨ What is FUR?

`fur` is a CLI tool for version-controlling your thoughts, one message at a time.

It creates threadable, forkable, jumpable timelines of ideas — like Git, but for your chat brain. Whether you’re journaling, brainstorming, or trying to reconstruct that *one* conversation from last Tuesday, `fur` gives you a navigable history of messages, markdown files, and madness.

You can:

- 🧵 Create and switch between threads
- ✍️ Jot down messages (inline or via Markdown files)
- ⏪ Jump through message history (past or branches)
- 🌳 View thread trees
- 🔍 Status check your current state
- 🐈 Cat linked Markdown content

---

## 🚀 Installation

You’ll need [Rust](https://www.rust-lang.org/tools/install).

```bash
cargo install fur-cli
```

---

## 🧪 Quickstart Tutorial

```bash
fur new                             # Start a new thread
fur jot -r user -t "Why are bees so weird?"      # Add a message
fur jot -r assistant -t "They're fuzzy anarchists with wings." 

fur jot -r user --file notes/bees.md             # Link a markdown file
fur cat                                          # View current message (file or text)

fur tree                                         # View thread structure
fur jump --past 1                                # Navigate backward
fur status                                       # See where you are
```

---

## 🧠 Message Modes

FUR supports two styles of jotting:

1. **Inline mode:**

   ```bash
   fur jot -r user -t "This is a short thought."
   ```

2. **Markdown file linking:**

   ```bash
   fur jot -r user --file path/to/note.md
   ```

Markdown files are stored *wherever you want* — we store the absolute path, not the content. You’re in charge of keeping the files alive.

---

## 📁 Where’s the data?

FUR creates a `.fur/` directory in your working folder:

* `.fur/index.json` – current state
* `.fur/threads/*.json` – each thread's structure
* `.fur/messages/*.json` – individual message metadata

---

## 🛣 Roadmap (v1.0)

* ✅ Thread creation
* ✅ Message jotting
* ✅ Jumping / Forking
* ✅ Markdown linking
* ✅ Tree view & status
* ⏳ Markdown rendering (future)
* ⏳ Thread export (future)

---

## 🐿️ Why is it called FUR?

FUR stands for Forkable, Unearthable, Recursive. It's a CLI-first tool for versioning your chat-style thought processes.

Also... if you shift your fingers one key to the left on a QWERTY keyboard while trying to type “git” — you get “fir”.


---

## 📜 License

MIT, like almost everything else that's friendly and open-source.
