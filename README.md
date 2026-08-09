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
  <a href="https://andrewcomputing.com/fur">Documentation, technical brief, and the case for FUR →</a>
</p>
---

**FUR: AI Conversation Archiving and Retrieval System**

FUR is a command-line system that transforms fragmented AI chats into structured, navigable, and fully searchable local archives.
Designed for researchers, scholars, developers, and writers who think with AI and need durable memory, clarity, and fast retrieval.

---

## Why FUR exists

AI chats vanish.
They get buried, lost, unsearchable, and unrecoverable.
FUR turns those conversations into structured knowledge you can return to, reuse, and build on.

---

> **Security notice for 1.x users.** A bug in `fur lock` meant conversation
> documents stored in subfolders were not encrypted. If you locked a project on
> an affected build, run `fur unlock` and `fur lock` again with this version.

---

## Why this matters beyond convenience

The 2.0 architecture is a direct answer to a transparency critique that applies to nearly every AI tool: **the record of what happened lives inside the tool that made it.** Chat histories sit in vendor databases, proprietary formats, or opaque local stores. When the tool changes, the account changes with it. When the tool disappears, so does the account.

FUR 2.0 inverts that. The archive is a folder of Markdown files with documented metadata. It can be read, verified, and audited by anyone with a text editor — no FUR binary, no database, no vendor, no network.

Concretely, this means:

**Inspectable without the tool.** Every conversation is plain text. `cat`, `grep`, `diff`, and `git log` all work on it directly. There is no step where you have to trust FUR's rendering of your own data, because you can read the underlying file.

**Verifiable provenance.** Each message carries an id, an author, and a timestamp in the document itself — not in a sidecar database that could drift or be regenerated. Attachments record a SHA-256 of their contents, so a document that has been altered since it was archived is detectable rather than assumed intact.

**Meaningful version control.** Because the canonical format is Markdown, Git produces readable diffs of how a line of reasoning changed, instead of opaque churn in serialized JSON. The history of the thinking is itself reviewable.

**No dependence on FUR.** Delete `.fur/`, uninstall the binary, and the archive is unaffected. `fur rebuild` reconstructs the index from the documents, which is the practical test that the documents — not the tool — hold the record.

For institutions, that combination addresses a real and growing problem. Research groups need to show how AI-assisted work was produced. Regulated organizations need records that outlive the software that created them and can be handed to an auditor unaltered. Archives and libraries need formats that will still open in twenty years. Legal and compliance functions need provenance they can attest to rather than infer.

None of that is satisfied by an export button. Export produces a copy whose fidelity you have to take on faith. FUR's Markdown *is* the record, and the round-trip guarantee — export, delete the index, rebuild, compare — is a property you can test yourself rather than a claim you have to accept.

To be precise about the scope: FUR makes the *record* of AI-assisted work transparent and auditable. It does not inspect model internals or explain why a model produced a given output. What it guarantees is that the conversation itself — what was asked, what came back, when, and by whom — remains legible and verifiable independently of any vendor, including this one.

---

## Overview

**Your conversations are plain Markdown files, and those files are the archive.**

A FUR project is a directory containing a `chats/` folder. Each conversation lives in its own subfolder, holding a spine document plus any long-form attachments.

```
my-research/
├── chats/
│   ├── peirce-icon-index-symbol-8f0c4a2e/
│   │   ├── convo.md                      # the conversation
│   │   └── CHAT-20260809-012744.md       # long-form attachment
│   └── derrida-on-saussure-1c9d55b0/
│       └── convo.md
│
└── .fur/                                 # rebuildable index — safe to delete
    ├── index.json
    ├── avatars.json
    ├── threads/
    └── messages/
```

A `convo.md` looks like this:

```markdown
---
fur_schema: 1
conversation_id: 8f0c4a2e-1b3d-4f5a-9c7e-2d8b6a1f0e33
title: Peirce on icon, index, symbol
created_at: 2026-08-09T01:23:14Z
tags:
  - semiotics
---

<!-- fur:msg id=3a7f9c21 avatar=andrew ts=2026-08-09T01:23:58Z -->

Is the icon/index distinction exhaustive, or does it presuppose a prior notion of resemblance?

<!-- fur:msg id=b2e14d80 avatar=gpt5 ts=2026-08-09T01:27:45Z link=CHAT-20260809-012744.md sha256=ab157403 -->
```

Readable as prose, parseable as data. The HTML comments are invisible in any Markdown renderer.

`.fur/` is a local index and cursor, not the record. Delete it and FUR rebuilds it from the documents:

```bash
rm -rf .fur
fur convo          # rebuilds automatically from chats/
```

That single property — *can I delete the database and recover everything from the readable files?* — is the design in one line. Copy a conversation folder to a USB stick, open it in Obsidian, email it, put it under Git; it stands on its own.

Core capabilities:

- Human-readable Markdown archive, canonical and portable
- Full-project search (`fur search`)
- Conversation tagging
- Timelines and message trees
- Jot mode and chat import
- Long-form attachments, kept inside the conversation folder
- `.frs` conversation scripting
- Project encryption (`fur lock` / `fur unlock`)
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

## Core Commands

### Create and Write

| Command                            | Description                              |
| ---------------------------------- | ---------------------------------------- |
| `fur new <name>`                   | Create a conversation                    |
| `fur jot "<text>"`                 | Add a short message                      |
| `fur jot "<text>" --file notes.md` | Add a message with a Markdown attachment |
| `fur chat [avatar]`                | Add long-form content                    |
| `fur msg`                          | Edit or delete a message                 |

Attachments passed with `--file` are copied into the conversation folder, so the archive stays self-contained.

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
| `fur convo --tag "speech acts"`            | Add spaced tag (normalized)         |
| `fur convo --clear-tags`                   | Remove all tags                     |
| `fur convo --delete <id>`                  | Permanently delete a conversation   |
| `fur clone [-i <id>] [--title <name>]`     | Deep-clone a conversation (full copy w/ Markdown)|
| `fur search <query>`                       | Full-project search                 |
| `fur search "peirce, saussure"`            | Multi-query search                  |

### Archive

| Command             | Description                                      |
| ------------------- | ------------------------------------------------ |
| `fur export`        | Write the active conversation into `chats/`      |
| `fur export --all`  | Write every conversation into `chats/`           |
| `fur rebuild`       | Reconstruct `.fur/` from `chats/`                |
| `fur onboard`       | Set which avatar is you, and pick faces          |
| `fur doctor`        | Repair missing or moved attachments              |

### Export

| Command       | Description                       |
| ------------- | --------------------------------- |
| `fur printed` | Export current thread to Markdown |
| `fur save`    | Export as `.frs` script           |
| `fur gsearch` | Scan all FUR journals on disk     |

`fur printed` writes a flattened, metadata-free copy at the project root — a printed record, not part of the archive.

### Security

| Command | Description |
|--------|-------------|
| `fur lock` | Encrypt the entire diary |
| `fur unlock` | Decrypt the diary |

FUR encrypts all conversations, messages, and Markdown attachments using AES-256-GCM.

Password input is hidden, and `fur lock` asks twice — a typo while locking would otherwise encrypt the archive under a passphrase nobody knows. `--hide` is accepted for compatibility and does nothing.

Locking writes a plaintext marker into `chats/`, so a project stays identifiable as locked even if `.fur/` is deleted. Commands report the lock rather than mistaking an encrypted archive for an empty directory.

---

## Example Workflow

```bash
# Create a project
mkdir research && cd research
fur new "Peirce and the Trichotomy of Signs"

# Add short notes
fur jot "Firstness/Secondness/Thirdness maps badly onto Saussurean dyads"

# Add longer content
fur chat gpt5

# Attach markdown notes
fur jot "Close reading" --file collected-papers-2-247.md

# Explore
fur convo
fur timeline
fur tree

# Search the entire archive
fur search "indexicality"
fur search "sign, referent"

# Export a printed record
fur printed
fur save session.frs
```

Every write updates `chats/` immediately — there's no separate save step.

---

## Portability

A conversation folder is the unit you move around:

```bash
cp -r chats/peirce-and-the-trichotomy-of-signs-8f0c4a2e ~/usb/
```

On another machine, drop it into any directory and run FUR:

```bash
mkdir -p ~/received/chats && cp -r ~/usb/peirce-and-the-trichotomy-of-signs-8f0c4a2e ~/received/chats/
cd ~/received && fur convo
```

FUR notices the documents, rebuilds `.fur/` from them, and reports what it found. Avatar names come from the documents; FUR then asks which one is you and offers faces, both skippable:

```bash
fur onboard
```

---

## Migrating from 1.x

Full release notes are in [CHANGELOG.md](CHANGELOG.md).

Existing projects keep working. `.fur/` is still read, and nothing is deleted or rewritten without asking.

To move a 1.x project into the new layout:

```bash
fur export --all
```

This writes every conversation into `chats/<slug>/` and copies attachments alongside them. Run it, read the output, and confirm it looks right — nothing is removed. `fur export` also lists any long-form files left in the `chats/` root that are now duplicated inside a conversation folder, so you can delete them yourself.

Two behaviour changes worth knowing:

- `fur chat` writes long-form files into the conversation folder instead of `chats/` root.
- `fur jot --file` copies the attachment into the conversation folder rather than recording an external path. Your original file is untouched; the archive keeps its own snapshot, and the recorded hash detects when the two drift apart.

---

## Search System

FUR's search engine inspects:

* Message text
* Attached Markdown files
* All conversations across the diary
* Flexible multi-query syntax
* Contextual snippet extraction

Examples:

```bash
fur search "arbitrariness of the sign"
fur search "peirce, interpretant"
fur search "trace, differance"
```

---

## Tagging System

Tags are stored at the conversation level and travel with the document:

```bash
fur convo --tag research
fur convo --tag "philosophy of language"
fur convo --tag "post-structuralism, close-reading"
fur convo --clear-tags
```

Normalization:

* lowercase
* trimmed
* spaces → hyphens

Example:
Input: `speech acts`
Stored as: `speech-acts`

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

The 2.0 architecture follows from the last of these. If nobody has the binary twenty years from now, the archive is still Markdown with documented metadata — readable by a person, parseable by anything.

A tool that asks you to trust it has made a claim. A tool you can verify without running it has made a guarantee. FUR aims at the second.

---

## Roadmap

Future enhancements include:

* Advanced editors for message modification
* Enhanced search output formats
* New export templates
* Cross-platform installers

---

## License

MIT License