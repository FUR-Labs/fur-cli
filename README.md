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

Unlike basic note-taking apps, FUR lets your conversations **branch**, like a tree. You can:

- **Fork** a message into multiple follow-ups.
- **Jump** backward or forward in time.
- **Write** or **link** Markdown files as messages.
- **See** your thread as a timeline or tree.
- **Track** everything locally, in simple JSON and Markdown files.

It's a **version control system for your thoughts** — like `git`, but for your chats and ideas.

> 🧠 _Imagine if you could save every ChatGPT message you've ever sent — explore different paths — and revisit any version like a "save point" in a game._

---

## 🌟 Why would I want this?

Here are some examples of how people use FUR:

- ✍️ **Writers**: Explore branching plotlines or rewrite drafts with different styles.
- 🧑‍💻 **Developers**: Track coding conversations with AI, test different solutions.
- 🧠 **Students**: Study a topic with AI, explore side questions without losing your place.
- 📚 **Researchers**: Organize chatbot responses and notes by topic and time.
- 🤯 **Overthinkers**: Save *every possible what-if*, and finally feel vindicated.

---

## 🛠 How does it work?

FUR runs in your terminal. You use commands like these:

```bash
# Start a new thread
fur new

# Add a message (yourself or AI)
fur jot -r user -t "What’s the deal with penguins?"
fur jot -r assistant -t "They can't fly, but they're great swimmers."

# Show the timeline of your current thread.
fur timeline [--verbose]
# - `--verbose`: Show full content of Markdown files linked in the messages. Without this flag, only file paths are shown.

# See the tree of all forks
fur tree

# Jump to a past message or fork
fur jump --past 1
fur jump --child 0

# Link a markdown file into your thread
fur jot -r user --file /home/me/chat-notes/penguins.md

# View your current status
fur status
```

All messages are saved locally, as plain `.json` and optional `.md` files, in a `.fur` folder in your project.

---

## 🔥 Why is it called FUR?

Because if you take the word `git` and move your fingers **one key to the left**, you get `fur`.

Also, it’s warm, soft, and good at storing memory.

---

### 🚀 Installation

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

## 🐾 Philosophy

FUR is minimal. It’s not meant to be an AI client. It’s a **memory tracker**.

It respects your brain’s tendency to wander and your desire to keep everything. It lets you **dig**, **fork**, **write**, and **retrace**.

The goal? Make it **easy to think recursively** and **keep track of yourself**.

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


## 📜 License

MIT, like almost everything else that's friendly and open-source.

