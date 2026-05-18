use std::error::Error;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use schemars::JsonSchema;

// Import our secure ObsidianVault from the vault module
use crate::vault::ObsidianVault;

// ============================================================================
// 1. Tool Input Schema Definition Structs
// ============================================================================

/// Arguments for the `create_note` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateNoteArgs {
    /// The title of the new note (can include subdirectories, e.g. "Work/Meeting").
    pub title: String,
    /// The Markdown content of the note.
    pub content: String,
    /// Optional list of tags to inject into the frontmatter.
    pub tags: Option<Vec<String>>,
}

/// Arguments for the `append_note` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct AppendNoteArgs {
    /// The title of the note to append to.
    pub title: String,
    /// The content to append to the bottom of the note.
    pub content: String,
}

/// Arguments for the `read_note` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReadNoteArgs {
    /// The title of the note to read.
    pub title: String,
}

/// Arguments for the `search_vault` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchVaultArgs {
    /// The full-text case-insensitive query string to search for.
    pub query: String,
}

// ============================================================================
// 2. MCP Schema and Result Payload Definitions
// ============================================================================

/// Represents a formal MCP Tool definition.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

/// A standard text segment returned as part of a CallToolResult.
#[derive(Debug, Serialize)]
pub struct TextContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}

/// The official Model Context Protocol response structure for tool execution.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CallToolResult {
    pub content: Vec<TextContent>,
    pub is_error: bool,
}

impl CallToolResult {
    /// Creates a successful tool execution payload.
    pub fn success(text: String) -> Self {
        Self {
            content: vec![TextContent {
                content_type: "text".to_string(),
                text,
            }],
            is_error: false,
        }
    }

    /// Creates a failed tool execution payload.
    pub fn error(err_msg: String) -> Self {
        Self {
            content: vec![TextContent {
                content_type: "text".to_string(),
                text: err_msg,
            }],
            is_error: true,
        }
    }
}

// ============================================================================
// 3. API & Router Functions
// ============================================================================

/// Helper function to generate a clean, serializable JSON Schema for a struct.
fn generate_schema<T: JsonSchema>() -> Value {
    let schema = schemars::schema_for!(T);
    serde_json::to_value(schema).unwrap_or(Value::Null)
}

/// Returns the formal names, schemas, and descriptions for all registered tools.
/// This allows client agents to discover how to interact with the Obsidian Vault.
pub fn get_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "create_note".to_string(),
            description: "Creates a new Markdown note in the Obsidian vault with date and tags frontmatter. Fails if the note already exists.".to_string(),
            input_schema: generate_schema::<CreateNoteArgs>(),
        },
        ToolDefinition {
            name: "append_note".to_string(),
            description: "Appends text with a timestamped header to the bottom of an existing note.".to_string(),
            input_schema: generate_schema::<AppendNoteArgs>(),
        },
        ToolDefinition {
            name: "read_note".to_string(),
            description: "Reads and returns the full text content of an existing note.".to_string(),
            input_schema: generate_schema::<ReadNoteArgs>(),
        },
        ToolDefinition {
            name: "search_vault".to_string(),
            description: "Performs a case-insensitive, full-text search across all notes, returning a list of relative matching file paths.".to_string(),
            input_schema: generate_schema::<SearchVaultArgs>(),
        },
    ]
}

/// Maps an incoming JSON-RPC tool invocation to the correct safe vault operation.
///
/// Deserializes inputs, executes the underlying filesystem methods, and maps all successes and
/// errors into a standardized MCP CallToolResult payload.
pub async fn handle_call_tool(
    vault: &ObsidianVault,
    name: &str,
    arguments: Value,
) -> Result<Value, Box<dyn Error + Send + Sync>> {
    let result = match name {
        "create_note" => {
            match serde_json::from_value::<CreateNoteArgs>(arguments) {
                Ok(args) => {
                    let tags = args.tags.unwrap_or_default();
                    match vault.create_note(&args.title, &args.content, tags).await {
                        Ok(_) => CallToolResult::success(format!("Successfully created note '{}'.", args.title)),
                        Err(e) => CallToolResult::error(format!("Error creating note: {}", e)),
                    }
                }
                Err(e) => CallToolResult::error(format!("Invalid arguments for 'create_note': {}", e)),
            }
        }
        "append_note" => {
            match serde_json::from_value::<AppendNoteArgs>(arguments) {
                Ok(args) => {
                    match vault.append_to_note(&args.title, &args.content).await {
                        Ok(_) => CallToolResult::success(format!("Successfully appended update to note '{}'.", args.title)),
                        Err(e) => CallToolResult::error(format!("Error appending to note: {}", e)),
                    }
                }
                Err(e) => CallToolResult::error(format!("Invalid arguments for 'append_note': {}", e)),
            }
        }
        "read_note" => {
            match serde_json::from_value::<ReadNoteArgs>(arguments) {
                Ok(args) => {
                    match vault.read_note(&args.title).await {
                        Ok(content) => CallToolResult::success(content),
                        Err(e) => CallToolResult::error(format!("Error reading note: {}", e)),
                    }
                }
                Err(e) => CallToolResult::error(format!("Invalid arguments for 'read_note': {}", e)),
            }
        }
        "search_vault" => {
            match serde_json::from_value::<SearchVaultArgs>(arguments) {
                Ok(args) => {
                    match vault.search_vault(&args.query).await {
                        Ok(results) => {
                            match serde_json::to_string(&results) {
                                Ok(json_str) => CallToolResult::success(json_str),
                                Err(e) => CallToolResult::error(format!("Failed to serialize search results: {}", e)),
                            }
                        }
                        Err(e) => CallToolResult::error(format!("Error searching vault: {}", e)),
                    }
                }
                Err(e) => CallToolResult::error(format!("Invalid arguments for 'search_vault': {}", e)),
            }
        }
        unrecognized => {
            CallToolResult::error(format!("Unrecognized tool name: '{}'.", unrecognized))
        }
    };

    Ok(serde_json::to_value(result)?)
}
