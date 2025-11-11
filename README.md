<!-- LOGO -->
<p align="center">
  <img src="https://github.com/user-attachments/assets/c3582cb8-c1cc-41ab-9ed1-f8fbde4d8c21" width="200" alt="fur logo"/>
</p>

<h1 align="center">FUR</h1>

<p align="center">
  <strong>Your AI conversation diary, organized and searchable</strong><br/>
  A simple tool that turns scattered chats into clean, browsable digital diaries.
</p>

<p align="center">
  👉 For a more visual overview, see the <a href="https://furchats.vercel.app/">FURChats portfolio site</a>.
</p>

---

## 🤔 What is FUR?

FUR is a lightweight tool that helps you **organize your AI chats into diaries** — each diary containing multiple conversations you can revisit, navigate, and export.

Instead of endless scrolling through chat history, FUR gives you:

✅ multiple diaries  
✅ each diary containing its own conversations  
✅ every conversation cleanly structured  
✅ easy navigation, timelines, and exports  
✅ memory you can actually *use* later

Think of it as **your AI companion's journal**, stored locally, under your control.

No cloud.  
No syncing.  
No mystery.  

Just clean **conversation tracking**, built for people who think with AI.

---

## 🌟 What can FUR do?

### **Write**
- **Jot** quick notes  
- **Chat** long-form messages or paste documents  
- **Attach Markdown** files  
- **Save** conversations as portable `.frs` scripts  

### **Organize**
- Create **multiple diaries** (`fur new`)  
- Assign **avatars/personas**  
- Jump to any message  
- View conversations as **timelines** or **trees**  
- Switch quickly between diaries  

### **Export**
- Save conversations as **Markdown**  
- Generate **PDF timelines**  
- Script entire conversations using `.frs`

### **Optional Git enhancements**  
If your FUR project lives inside a Git repo, you also get:

- `fur status` showing Git status  
- passthrough commands (`fur add`, `fur commit`, etc.)

...but Git is **never required**.  
FUR works perfectly without it.

---

## 🛠 How it works

FUR stores everything inside a small local folder:

* `.fur/index.json` → list of diaries  
* `.fur/threads/*.json` → one file per diary  
* `.fur/messages/*.json` → messages inside each diary  
* `.fur/avatars.json` → your persona mapping  

Everything is **human-readable**, portable, and version-safe.

---

## 📘 Example workflow

```bash
# Create a new AI diary
fur new "My GPT-5 Notes"

# Add quick jot
fur jot "Deep learning troubleshooting ideas"

# Paste long chats or Markdown
fur chat gpt5
fur jot andrew --file notes/forecasting.md

# View as timeline
fur timeline --contents

# Export to Markdown or PDF
fur printed --out diary.md
fur printed --out diary.pdf

# Switch to another diary (alias 'thread')
fur convo list
fur convo switch research

# Optional: Git passthrough (if repo exists)
fur add .
fur commit -m "Updated diaries"
fur push
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

FUR isn't trying to be an AI client or a chat interface.

It's something simpler:

**A place to keep your conversations with clarity.**

Your chats — with AI, with yourself, with collaborators — deserve structure.
FUR gives them a home.

*Fast.*
*Local.*
*Searchable.*
*Yours.*

---

## 🛣 Roadmap to v1.0

✅ **Already complete**

* Multiple diaries & conversations
* Avatars / personas
* Timelines & trees
* Jot & chat modes
* Jump navigation
* `.frs` scripting
* Markdown & PDF exports
* Optional Git passthrough

🔜 **Planned**

* Message deletion & editing
* Diary-level tagging
* Richer export layouts
* Search improvements
* Optional helper for Git ignore rules
* Cross-platform installers

---

## 📜 License

MIT, like almost everything else that's friendly and open-source.
