# 🖥️ Obsidian MCP Human Installation Guide

Welcome! This guide is designed for humans to easily set up and configure the **Obsidian MCP Server** on their system in **less than 2 minutes**.

Rust's compiled nature means this server is packaged as a **single, standalone, native executable** (`obsidian_mcp.exe` on Windows).
- **Zero Software Requirements:** No Node.js, Python, Rust, or Cargo dependencies are needed to run the server!
- **Instant Setup:** Just point your AI agent to the pre-compiled binary in the `bin/` folder.

---

## 🛠️ Step-by-Step Integrations

### 👾 1. Installing into Antigravity AI (Recommended)

Antigravity uses a centralized configuration file to manage all of its active AI tools.

1. **Locate your Antigravity config file:**
   - Press `Win + R` on your keyboard, type `%USERPROFILE%\.gemini\antigravity`, and hit Enter.
   - You will see a file named `mcp_config.json`.

2. **Open & Edit `mcp_config.json`:**
   - Open this file in your favorite text editor (like Notepad, VS Code, or Cursor).

3. **Register the Obsidian Server:**
   - Inside the `"mcpServers"` block, add the following section. Ensure you customize the path to your actual repository directory and your target Obsidian Vault directory:

   ```json
   "obsidian": {
       "command": "C:\\Antigravity projects\\Rust\\obsidian_mcp\\bin\\obsidian_mcp.exe",
       "args": [],
       "env": {
           "OBSIDIAN_VAULT_PATH": "G:\\My Drive\\DriveSyncFiles\\Obsidian\\Obsidian"
       }
   }
   ```

   > [!WARNING]
   > **JSON Path Syntax Rule:**
   > Windows uses backslashes (`\`) for folders. However, inside JSON files, backslashes **must** be double-escaped as `\\`. 
   > - **Correct:** `C:\\Antigravity projects\\Rust\\obsidian_mcp\\bin\\obsidian_mcp.exe`
   > - **Incorrect:** `C:\Antigravity projects\Rust\obsidian_mcp\bin\obsidian_mcp.exe`

4. **Save & Refresh:**
   - Save the file and restart/refresh your Antigravity session. The agent will immediately detect your vault tools!

---

### 🛰️ 2. Installing into Cursor IDE

To give Cursor's Composer and chat agents full access to your Obsidian vault notes:

1. Open Cursor and click the **Gear Icon** in the top-right corner to open Settings.
2. Navigate to **Cursor Settings** > **Features** > **MCP**.
3. Click the **+ Add New MCP Server** button.
4. Input the following details:
   - **Name:** `obsidian-mcp`
   - **Type:** `command`
   - **Command:** `C:\Antigravity projects\Rust\obsidian_mcp\bin\obsidian_mcp.exe "G:\My Drive\DriveSyncFiles\Obsidian\Obsidian"`
5. Click **Save**. The status circle will instantly turn **green**, indicating Cursor has successfully established the connection!

---

### 💬 3. Installing into Claude Desktop

To give the official Claude Desktop client the ability to read, write, and search your notes:

1. Press `Win + R`, paste `%APPDATA%\Claude\`, and press Enter.
2. Locate or create a file named `claude_desktop_config.json` and open it.
3. Paste the following configuration:

```json
{
  "mcpServers": {
    "obsidian-mcp": {
      "command": "C:\\Antigravity projects\\Rust\\obsidian_mcp\\bin\\obsidian_mcp.exe",
      "args": [],
      "env": {
        "OBSIDIAN_VAULT_PATH": "G:\\My Drive\\DriveSyncFiles\\Obsidian\\Obsidian"
      }
    }
  }
}
```
4. Restart your Claude Desktop application. You will see a new **Hammer Icon** under the text input bar representing the active Obsidian vault tools!

---

## 🔍 Troubleshooting & FAQs

### ❓ I get a "Command not found" or "Execution failed" error?
- Double-check that the file path to `obsidian_mcp.exe` inside your JSON configuration matches your exact folder location.
- Verify you used double-escaped `\\` inside `mcp_config.json` or `claude_desktop_config.json`.

### ❓ Does the Obsidian app need to be open?
- **No.** The MCP server interacts directly with the text files on your storage drive. You do not need to have the Obsidian desktop application running to query or update your vault.

### ❓ Can I put my vault on a network drive or Google Drive?
- **Yes!** As long as the directory is synced locally and has an absolute Windows drive path (e.g. `G:\My Drive\DriveSyncFiles\Obsidian`), the server can read and write to it securely.
