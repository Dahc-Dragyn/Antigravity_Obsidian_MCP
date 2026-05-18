# 🤖 Obsidian MCP AI Agent System Manual

Hello, AI Agent! If you are reading this file, it means you have been integrated with the **Obsidian MCP Server** to manage and curate the human user's knowledge vault. 

This document serves as your **specialized context injection manual**. Read and follow these directives to ensure high-fidelity note manipulation, clean structural indexing, and smooth agentic performance.

---

## 🛠️ Rapid Tool Reference

You have access to four core sandboxed file operations:

| Tool Name | Parameters | Direct Action | Agent Recovery Path |
| :--- | :--- | :--- | :--- |
| `create_note` | `title` (str), `content` (str), `tags` (array, opt) | Generates a new note with YAML frontmatter. | If it errors with *AlreadyExists*, fall back to `append_note`. |
| `append_note` | `title` (str), `content` (str) | Appends text to the bottom under a timestamped header. | If it errors with *NotFound*, run `search_vault` to find the correct title. |
| `read_note` | `title` (str) | Reads the full text of a target note. | If the fast path fails, the server will recursively scan the vault for you. |
| `search_vault` | `query` (str) | Case-insensitive full-text search across all `.md` files. | Use this to audit files or verify if a topic exists before creating. |

---

## 📋 Behavioral Directives & Best Practices

### 1. 🗂️ Knowledge Graph Curation (`[[WikiLinks]]`)
- When creating or editing notes, connect topics logically!
- Use standard Obsidian **Internal WikiLinks** to build a rich associative knowledge graph for the human user.
  - *Example:* "For more details on tactical responses, refer to [[Tactical HUD Controls]]."
  - *Example:* "Review the daily brief in [[Inbox/Daily Briefing]]."

### 2. 📝 Formatting Elegance
- Always write robust, beautifully structured Markdown. 
- Use headers (`#`, `##`, `###`), bold styling, itemized tables, bullet points, and code blocks to maximize readability.
- **Do not write inline placeholders** or leave tasks incomplete.

### 3. 🏷️ YAML Frontmatter Operations
- The server's `create_note` tool automatically handles injecting the creation date and formatting a clean, bulletproof YAML frontmatter header block:
  ```yaml
  ---
  date: YYYY-MM-DD
  tags:
    - "tag1"
    - "tag2"
  ---
  ```
- **Action:** Pass tags as a standard array of strings in the `tags` argument. You do **not** need to manually format the frontmatter header in the `content` body unless you are adding additional custom properties (e.g., `status: in-progress`).

### 4. 🔤 Title and Case-Sensitivity Strategies
- Note titles are resolved relative to the vault root.
- The server automatically handles appending the `.md` extension.
- **Case-Insensitive Fallback:** The server runs a WalkDir search if a direct read fails. However, to maximize speed and efficiency, keep a memory or list of exact note titles you discover in the vault to execute instant direct reads.

---

## 🛡️ Sandbox Boundaries & Security Limits

- **No Traversal:** Rejects drive letters (`C:\`), absolute paths (`/`), or parent directories (`..`). Do not try to escape the vault; the server will immediately drop the request and throw a traversal exception.
- **Enforced Markdown:** You can only write to `.md` files. Attempts to manipulate `.json`, `.exe`, `.bat`, or system files are structural impossibilities.
- **Reserved Names:** Rejects Windows reserved names (`CON`, `PRN`, `AUX`, `NUL`, etc.). Avoid naming notes with these exact device components.
