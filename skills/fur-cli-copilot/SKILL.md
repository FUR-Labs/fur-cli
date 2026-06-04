---
name: fur-cli-copilot
metadata: 
  version: "1.1"
description: >
  Copilot skill for fur-cli — an AI conversation archiving and retrieval system.
  Load this skill whenever a user asks for help with `fur` commands, workflows,
  project structure, scripting, search, encryption, or any fur-related task.
  Install: cargo install fur-cli
  Source:  https://github.com/andrewrgarcia/fur-cli
---

# FUR CLI — AI Copilot Skill

## What FUR is

FUR turns AI chat sessions into a structured, searchable, local diary.
It is NOT a chat client. It is a durable memory system.
Storage is plain JSON files inside a `.fur/` directory at the project root.
All data is local, offline, and transparent.

---

## Mental Model

```
project-folder/
 ├── .fur/
 │    ├── index.json          # master state: active thread, thread list
 │    ├── avatars.json        # persona registry { "main": "me", "me": "🦊", "ai": "🤖" }
 │    ├── lock.json           # present only when encrypted
 │    ├── .lockcheck          # password verification blob (encrypted)
 │    ├── threads/<uuid>.json # per-conversation metadata + message ID list
 │    └── messages/<uuid>.json# per-message payload
 └── chats/                  # markdown attachments live here (or anywhere)
```

Key concepts:
- **Conversation (thread)**: a named container holding an ordered list of message IDs.
- **Message**: atomic unit. Has avatar, text OR markdown path, optional image, parent, children, branches.
- **Avatar**: named persona. `main` is a pointer to the user's own avatar key.
- **Active thread**: the currently selected conversation (stored in `index.json["active_thread"]`).
- **Current message**: cursor position within a conversation (stored in `index.json["current_message"]`).
- **Branch**: a divergent reply chain off a parent message (stored in `msg["branches"]`).
- **Schema version**: currently `0.3`. Auto-migrated on startup if older messages are detected.

---

## ★ Core Commands (Start Here)

These are the commands people use every day. Suggest these first before reaching for anything else.

---

### `fur new "<conversation name>"`

**Two roles depending on context:**

```bash
# First time in a new directory → initializes .fur/ diary AND creates first conversation
mkdir my-project && cd my-project
fur new "GPT-5 Experiments"
# Prompts for avatar setup (press Enter for defaults: me / ai)

# In an existing diary → just creates a new conversation and switches to it
fur new "Follow-up Session"
```

---

### `fur jot`

Add a short message to the active conversation. The workhorse command.

```bash
fur jot "My observation or thought"          # jot as yourself (main avatar)
fur jot ai "The model responded with..."     # jot as the 'ai' avatar
fur jot --text "My thought"                  # explicit flag, avoids ambiguity
fur jot --file notes.md                      # attach a markdown file as the message body
fur jot ai --file long_response.md           # attach as a specific avatar
```

Avatar resolution for `fur jot [arg1] [arg2]`:

| arg1             | arg2   | Result                              |
|------------------|--------|-------------------------------------|
| known avatar     | text   | jot as arg1, text = arg2            |
| known avatar     | (none) | jot as arg1, text from --text flag  |
| unknown string   | (none) | jot as main avatar, text = arg1     |
| (none)           | (none) | jot as main avatar, text from --text|

When in doubt, use `--text` to be explicit.

---

### `fur chat`

For long-form content — paste a full AI response or write multi-line text. Opens stdin capture mode.

```bash
fur chat           # captures as main avatar
fur chat gpt5      # captures as 'gpt5' avatar
# Finish input with Ctrl+D (Linux/macOS) or Ctrl+Z then Enter (Windows)
# Prompts to save as a .md file, then attaches it to the conversation
```

Use `fur chat` when the content is too long to quote inline. Use `fur jot --file` when the markdown already exists on disk.

---

### `fur convo`

Manage and navigate conversations.

```bash
fur convo                  # list all conversations (shows ~7 around the active one)
fur convo --all            # show full list when the diary gets large
fur convo <hash>           # switch the active conversation by ID prefix (8 chars is enough)
fur convo --rename "New Title"          # rename the active conversation
fur convo <hash> --rename "New Title"   # rename by prefix
fur convo --tag research                # add a tag to the active conversation
fur convo --tag "deep learning"         # stored as deep-learning
fur convo --tag "ml, forecasting"       # add multiple tags at once
fur convo --untag research              # remove a tag
fur convo --clear-tags                  # remove all tags
fur convo --delete                      # delete the active conversation (destructive, double-confirmed)
fur convo <hash> --delete               # delete a specific conversation
```

The active conversation is highlighted in the list. Switching with `fur convo <hash>` changes where `fur jot`, `fur show`, `fur tree`, etc. operate.

---

### `fur tree`

The analog of `git status` for a conversation. Shows the full branching structure of the active conversation — every message, its avatar, and how replies branch off each other. Use this to get the big picture before writing or navigating.

```bash
fur tree
```

Output is a visual tree:
```
🌳 Conversation Tree: GPT-5 Experiments
├── 🦊 [me] Starting my research on... abc12345
│      └── 🤖 [ai] Here is what I found... def67890
└── 🦊 [me] Follow-up question... ghi11111
```

---

### `fur show`

Show the full conversation in the terminal, with all message text expanded. The go-to command for reading back what you've written.

```bash
fur show           # full conversation, verbose, with markdown attachment contents
```

`fur show` is identical to `fur timeline --verbose`. If you only want the message headers (no attachment contents), use `fur timeline` instead.

---

### `fur printed`

Export the active conversation as a Markdown file (or PDF).

```bash
fur printed                    # → CONVERSATION_TITLE_<shortid>.md  (auto-named)
fur printed my_report.md       # export to a specific filename
fur printed my_report.pdf      # export as PDF (requires pdflatex in PATH)
```

The Markdown export includes word count and token estimate at the top. Good for handing off to another AI, archiving, or sharing.

---

### `fur avatar` / `fur avatar new`

Manage the personas used in conversations.

```bash
fur avatar           # list all avatars with their emoji and message counts
fur avatar new       # interactive wizard to create a new avatar
```

Avatars are stored in `.fur/avatars.json`. The `main` key is a special pointer to your own avatar — when you `fur jot` without specifying an avatar, it uses `main`.

```json
{
  "main": "me",
  "me":   "🦊",
  "ai":   "🤖",
  "boss": "😤"
}
```

---

## Typical Daily Workflow

```bash
# First time: initialize
mkdir research && cd research
fur new "LLM Benchmarking"       # sets up .fur/ and first conversation

# Write messages
fur jot "Testing GPT-5 on coding tasks"
fur jot ai "Results: pass@1 = 0.87, pass@10 = 0.94"
fur jot --file analysis.md       # attach longer notes

# Navigate and inspect
fur tree                          # see full structure
fur show                          # read everything back
fur convo                         # check which conversations exist

# Create a second conversation, come back to first
fur new "Literature Review"
fur jot "Smith 2024 argues transformers are..."
fur convo                         # list all
fur convo b3f2a1                  # switch back to LLM Benchmarking by prefix

# Export
fur printed                       # → LLM_BENCHMARKING_b3f2a1c4.md
```

---

## All Other Commands (Reference)

### Writing & Editing

| Intent | Command |
|---|---|
| Edit a message | `fur msg <prefix> --edit "new text"` |
| Edit interactively ($EDITOR) | `fur msg <prefix> --edit --interactive` |
| Change message avatar | `fur msg <prefix> --edit --avatar ai` |
| Delete a message | `fur msg <prefix> --delete` |
| Insert before a message | `fur msg <prefix> --pre jot me "inserted"` |
| Insert after a message | `fur msg <prefix> --post jot ai "inserted"` |

### Navigation

| Intent | Command |
|---|---|
| Timeline (headers only) | `fur timeline` |
| Timeline with attachment contents | `fur timeline --contents` |
| Jump back N messages | `fur jump --past N` |
| Jump to child N | `fur jump --child N` |
| Jump to specific message ID | `fur jump --id <full-uuid>` |

### Search

| Intent | Command |
|---|---|
| Search all conversations | `fur search "query"` |
| Multi-query (OR logic) | `fur search "term1, term2"` |
| Limit results per conversation | `fur search "query" --limit 3` |
| JSON output | `fur search "query" --json` |
| Scan all fur projects on disk | `fur gsearch` |
| Scan from specific directory | `fur gsearch --dir ~/projects` |
| Control scan depth | `fur gsearch --depth 3` |

Search inspects: message text and attached markdown files, across all conversations.

### Export

| Intent | Command |
|---|---|
| Export as .frs script | `fur save` |
| Export .frs to named file | `fur save --out session.frs` |

### Cloning

| Intent | Command |
|---|---|
| Clone active conversation | `fur clone` |
| Clone with custom title | `fur clone --title "Fork of X"` |
| Clone by ID prefix | `fur clone -i <prefix>` |
| Cross-project clone | `fur xclone --to ../other-project` |
| Cross-project with title | `fur xclone --to ../other-project --title "Copy"` |

`fur clone` creates new UUIDs for all messages and copies markdown attachments. The original is untouched.

### Security (Encryption)

| Intent | Command |
|---|---|
| Encrypt diary | `fur lock` |
| Encrypt (hidden input) | `fur lock --hide` |
| Decrypt diary | `fur unlock` |
| Decrypt (hidden input) | `fur unlock --hide` |
| Generate strong passphrase | `fur keygen` |
| Generate N-word passphrase | `fur keygen --words 8` |

Encryption: AES-256-GCM on all files in `.fur/messages/`, `.fur/threads/`, `.fur/index.json`, `.fur/.lockcheck`, and `chats/`. Most commands refuse to run while locked.

### Repair

| Intent | Command |
|---|---|
| Check for broken attachments | `fur doctor` |
| Deep search (home dir) | `fur doctor --deep` |
| Remove unrecoverable metadata | `fur doctor --clean` |

Doctor finds moved markdown files by filename + SHA-256 hash match. Use this whenever a file was moved or renamed after being attached.

### Git Passthrough

```bash
fur status         # fur conversation status + git status (if in a git repo)
fur add .
fur commit -m "snapshot"
fur push
fur pull
```

---

## Scripting (.frs Files)

Define conversations declaratively. Run with `fur script.frs` or `fur run script.frs`.

```frs
# Comments start with #
new "Conversation Title"

jot me "Hello from me"
jot ai "Hello from the AI"
jot me --file notes.md

branch {
    jot me "Branch A response"
    jot ai "Branch A AI reply"
}

store           # persists to .fur/ — required exactly once

timeline --contents
printed --out output.pdf
tree
status
```

Rules: `store` must appear exactly once. Commands before `store` run on an in-memory thread. `branch {}` creates a divergent message chain off the last message.

---

## ID Prefix System

All IDs are full UUIDs. FUR accepts any unambiguous prefix:
- `fur convo abc123` — switches to the conversation whose ID starts with `abc123`
- `fur msg abc --delete` — deletes the message whose ID starts with `abc`
- Ambiguous prefix → error listing all matches. Use more characters.

8 characters is almost always enough in practice.

---

## Error Patterns & Fixes

| Error | Cause | Fix |
|---|---|---|
| `🚨 .fur/ not found` | Not in a fur project dir | `cd` to project root; or `fur new "Title"` to initialize |
| `🔒 Project locked` | Diary is encrypted | `fur unlock` |
| `❌ No active conversation` | No thread selected | `fur convo <id>` to switch |
| `❌ Ambiguous prefix` | Prefix matches multiple IDs | Use more characters |
| `⚠️ Script finished without a store` | .frs missing `store` | Add `store` line to the script |
| `❌ No message matches prefix` | Wrong prefix or wrong convo | `fur convo` to check, `fur tree` to inspect |
| `pandoc` error during PDF | pandoc not installed | Install pandoc, or export as `.md` |
| `pdflatex` error during PDF | pdflatex not installed | Install TeX Live / MikTeX |

---

## Key Constraints

- **One `.fur/` per project directory.** Each directory is an independent diary.
- **`fur gsearch`** scans all fur projects on disk. `fur search` searches only the current diary.
- **Markdown attachments** are referenced by path. Moving files breaks the link — use `fur doctor` to repair.
- **Encryption scope**: covers `chats/` but NOT arbitrary markdown paths outside it.
- **`fur show`** = `fur timeline --verbose`. Both render the full conversation with attachment contents.
- **Schema migration** runs automatically on startup — non-destructive.

---

## Copilot Behavior Guidelines

When assisting a user with FUR:

1. **Lead with core commands.** `fur new`, `fur jot`, `fur chat`, `fur convo`, `fur tree`, `fur show`, `fur printed`, `fur avatar` cover 90% of use cases. Reach for others only when needed.
2. **Confirm the working directory** before suggesting commands. FUR operates on `.fur/` in the current directory.
3. **Prefer explicit avatar flags** (`fur jot ai "..."`) over positional resolution to avoid ambiguity.
4. **Use ID prefixes** — 8 characters is enough. Never ask for or paste full UUIDs.
5. **`fur tree` first** when a user is confused about conversation structure. It shows everything.
6. **`fur convo --all`** when the user can't find a conversation — the default list truncates around the active thread.
7. **For export**, `fur printed` is the default. Ask about PDF only if they specifically need it (requires pdflatex).
8. **`fur doctor`** is the answer to any "I moved my files and now things are broken" situation.
9. **If the diary is locked**, all commands fail — `fur unlock` must come first.
10. **Cloning is safe** — `fur clone` creates entirely new UUIDs and copies files; the original is untouched.