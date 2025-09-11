<!-- LOGO -->
<p align="center">
  <img src="https://github.com/user-attachments/assets/c3582cb8-c1cc-41ab-9ed1-f8fbde4d8c21" width="200" alt="fur logo"/>
</p>

<h1 align="center">FUR Examples</h1>

This folder showcases how FUR handles very different scenarios, from minimal humor to serious corporate workflows to maximal stress-tests of branching logic.  

The goal: demonstrate **versatility** and push FUR to its limits.

---

## Core Demos

### 🐣 Dad Jokes
- **Type:** Minimal demo  
- **What it shows:** How to use FUR for the simplest possible branching structure.  
- **Why it’s here:** A baseline test. If FUR can’t handle branching punchlines, it can’t handle anything bigger.  


### 🗂 Department Meeting
- **Type:** Realistic corporate use case  
- **What it shows:** A structured conversation across multiple avatars (finance, ops, HR), with hand-off to an attached Markdown report.  
- **Why it’s here:** Demonstrates practical business applications of FUR — linking documentation into the thread and exporting professional notes.  


### 🐧 The Penguin Verses
- **Type:** Maximal stress test  
- **What it shows:** Alien penguin encounters, recursive branches, absurd linked documents (*Meeponomicon*, *The Final Pound*).  
- **Why it’s here:** A sandbox to see how far the tool could go before breaking. Lots of nesting, cross-file linking, and surreal content.  


### ⚛️ Quantum Playground
- **Type:** Educational / technical demo  
- **What it shows:** One conversation that branches into multiple “vibes” — cinematic, mathematical, startup pitch — each pulling in its own linked doc.  
- **Why it’s here:** Tests how FUR handles multiple branches leading to different narrative styles, then merges them back into a single journal.  

---
## 📂 Supporting Markdown Files

These aren’t `.frs` scripts themselves. Instead, they live in [`examples/docs/`](./docs) and [`examples/chats/`](./chats), and are **called by the `.frs` demos** (using `--file`) to inject full documents into the flow.  

When you run `fur timeline --contents` or export to PDF, these files are printed *in full*, embedded in the conversation.  

---

### [`examples/docs/`](./docs)
- **[MONTHLY_SUMMARY_REPORT.md](./docs/MONTHLY_SUMMARY_REPORT.md)**  
  - Linked from: `department_meeting.frs`  
  - Purpose: Simulates a corporate handoff (finance/HR meeting summary).  

---

### [`examples/chats/`](./chats)
- **[CINEMATIC_ENTANGLEMENT.md](./chats/CINEMATIC_ENTANGLEMENT.md)**  
  - Linked from: `quantum_playground.frs`  
- **[ENTANGLEMENT_EQS.md](./chats/ENTANGLEMENT_EQS.md)**  
  - Linked from: `quantum_playground.frs`  
- **[QUANTUM_STARTUP_PITCH.md](./chats/QUANTUM_STARTUP_PITCH.md)**  
  - Linked from: `quantum_playground.frs`  
- **[QUANTUM_MANIFESTO.md](./chats/QUANTUM_MANIFESTO.md)**  
  - Linked from: `quantum_playground.frs`  

- **[MEEPONOMICON.md](./chats/MEEPONOMICON.md)**  
  - Linked from: `penguin_verses.frs`  
- **[THE_FINAL_POUND.md](./chats/THE_FINAL_POUND.md)**  
  - Linked from: `penguin_verses.frs`  

## Why so eclectic?

Because FUR is meant to be **general**:  
- **Minimal demos** prove it works.  
- **Corporate scenarios** prove it’s useful.  
- **Absurd stress-tests** prove it’s resilient.  

This mix is deliberate — it shows that FUR is not just a toy, not just a business tool, but a flexible system for *any* conversation or narrative.
