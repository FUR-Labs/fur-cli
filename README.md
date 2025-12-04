<!-- LOGO -->
<p align="center">
  <img src="https://github.com/user-attachments/assets/c3582cb8-c1cc-41ab-9ed1-f8fbde4d8c21" width="200" alt="fur logo"/>
</p>

<h1 align="center">FUR</h1>

<p align="center">
  <a href="https://crates.io/crates/fur-cli"><img src="https://img.shields.io/crates/v/fur-cli.svg" /></a>
  <a href="https://github.com/andrewrgarcia/fur-cli"><img src="https://img.shields.io/github/stars/andrewrgarcia/fur-cli" /></a>
  <a href="#"><img src="https://img.shields.io/badge/platform-linux%20%7C%20macos%20%7C%20windows-blue" /></a>
  <a href="#"><img src="https://img.shields.io/badge/status-stable-green" /></a>
</p>

<p align="center">
  <strong>Your AI conversation diary — organized, searchable, and local.</strong><br/>
  Turn scattered chats into clean, structured, browsable digital diaries.
</p>


<p align="center">
  For an optional visual walkthrough, see the <a href="https://furchats.vercel.app/">FURChats portfolio site</a>.
</p>

---

**FUR: AI Conversation Archiving and Retrieval System**

FUR is a command-line system that transforms fragmented AI chats into structured, navigable, and fully searchable local archives.  
Designed for researchers, developers, and writers who think with AI and need durable memory, clarity, and fast retrieval.

---

## Why FUR exists

AI chats vanish.  
They get buried, lost, unsearchable, and unrecoverable.  
FUR turns those conversations into structured knowledge you can return to, reuse, and build on.

---

## Overview

FUR stores every AI conversation in a clean local directory.  
Each project (a diary) contains multiple conversations, and each conversation contains timestamped messages, metadata, trees, timelines, and optional Markdown attachments.

Core capabilities:

- Local, transparent JSON storage  
- Full-project search (`fur search`)  
- Conversation tagging  
- Timelines and message trees  
- Jot mode and chat import  
- Markdown attachments and export  
- `.frs` conversation scripting  
- Fast, portable, offline

---

## Installation

### From crates.io

```bash
cargo install fur-cli
```

### From source

```bash
cargo install --path . --force
```

---

## Project Structure

```
.fur/
 ├── index.json            # master index
 ├── avatars.json          # persona aliases
 ├── threads/              # per-conversation metadata
 └── messages/             # per-message storage
```

All files are plain JSON or Markdown.

---

## Core Commands

### Create and Write

| Command                            | Description                              |
| ---------------------------------- | ---------------------------------------- |
| `fur new <name>`                   | Create a conversation                    |
| `fur jot "<text>"`                 | Add a short message                      |
| `fur jot "<text>" --file notes.md` | Add a message with a Markdown attachment |
| `fur chat [avatar]`                | Add long-form content                    |
| `fur msg`                          | Edit or delete a message                 |

### Navigate

| Command                 | Description                |
| ----------------------- | -------------------------- |
| `fur convo`             | List conversations         |
| `fur convo <id>`        | Switch active conversation |
| `fur timeline`          | Chronological timeline     |
| `fur tree`              | Message tree               |
| `fur jump <message-id>` | Jump to a message          |

### Organize

| Command                                    | Description                         |
| ------------------------------------------ | ----------------------------------- |
| `fur convo --tag research`                 | Add a tag                           |
| `fur convo --tag "deep learning"`          | Add spaced tag (normalized)         |
| `fur convo --clear-tags`                   | Remove all tags                     |
| `fur convo --delete <id>`                  | Permanently delete a conversation   |
| `fur search <query>`                       | Full-project search                 |
| `fur search "deep learning, optimization"` | Multi-query search                  |

### Export

| Command       | Description                       |
| ------------- | --------------------------------- |
| `fur printed` | Export current thread to Markdown |
| `fur save`    | Export as `.frs` script           |
| `fur gsearch` | Scan all FUR journals on disk     |


---

## Example Workflow

```bash
# Create a project
mkdir research && cd research
fur new "GPT-5 Experiments"

# Add short notes
fur jot "Symbolic regression tests using KAN"

# Add longer content
fur chat gpt5

# Attach markdown notes
fur jot "Derivations" --file derivations.md

# Explore
fur convo
fur timeline
fur tree

# Search the entire archive
fur search "memory architecture"
fur search "deep, learning"

# Export
fur printed
fur save session.frs
```

---

## Search System (v1.0)

FUR’s search engine inspects:

* Message text
* Attached Markdown files
* All conversations across the diary
* Flexible multi-query syntax
* Contextual snippet extraction

Examples:

```bash
fur search "universal approximator"
fur search "symbolic regression, metadata"
fur search "deep, learning"
```

---

## Tagging System (v1.0)

Tags are stored at the conversation level:

```bash
fur convo --tag research
fur convo --tag "neural forecasting"
fur convo --tag "macro-modeling, metadata-tools"
fur convo --clear-tags
```

Normalization:

* lowercase
* trimmed
* spaces → hyphens

Example:
Input: `deep learning`
Stored as: `deep-learning`

---

## Philosophy

FUR is not a chat client.
It is a durable memory system for people who think and work with AI as part of their intellectual workflow.

Principles:

* Local ownership
* Transparent formats
* Reliability
* Portability
* Speed
* Long-term retrieval

---

## Roadmap

Future enhancements include:

* Advanced editors for message modification
* Enhanced search output formats
* New export templates
* Optional encrypted diaries
* Cross-platform installers

---

## License

MIT License
