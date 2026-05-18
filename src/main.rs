mod vault;
mod server;

use std::error::Error;
use std::env;
use tokio::io::{stdin, BufReader, AsyncBufReadExt, AsyncWriteExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use vault::ObsidianVault;
use server::{get_tool_definitions, handle_call_tool};

// ============================================================================
// 1. JSON-RPC Request & Response Payloads
// ============================================================================

/// Represents a standard incoming JSON-RPC 2.0 Request Frame.
#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<Value>, // id can be numeric, string, or null
    method: String,
    params: Option<Value>,
}

/// Represents a standard outgoing JSON-RPC 2.0 Response Frame.
#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

/// A standard JSON-RPC 2.0 Error object.
#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

// ============================================================================
// 2. Request Processing
// ============================================================================

/// Parses and routes an individual line as a JSON-RPC request.
///
/// Returns `Some(JsonRpcResponse)` if the incoming frame is a Request requiring
/// a response, or `None` if it is a Notification (e.g. `notifications/initialized`).
async fn process_line(
    line: &str,
    vault: &ObsidianVault,
) -> Option<JsonRpcResponse> {
    let req: JsonRpcRequest = match serde_json::from_str(line) {
        Ok(r) => r,
        Err(e) => {
            return Some(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: None,
                result: None,
                error: Some(JsonRpcError {
                    code: -32700, // Parse Error
                    message: format!("Parse error: {}", e),
                    data: None,
                }),
            });
        }
    };

    // Standard JSON-RPC Notification Check: notifications do not have an ID and get no response
    let id = match &req.id {
        Some(id_val) => id_val.clone(),
        None => return None,
    };

    match req.method.as_str() {
        // Required protocol handshake: informs client of version and capabilities
        "initialize" => {
            let result = serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "obsidian-vault-mcp",
                    "version": "0.1.0"
                }
            });
            Some(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: Some(id),
                result: Some(result),
                error: None,
            })
        }
        // Informs the client which tools this MCP server implements
        "tools/list" => {
            let tools = get_tool_definitions();
            let result = serde_json::json!({
                "tools": tools
            });
            Some(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: Some(id),
                result: Some(result),
                error: None,
            })
        }
        // Executes a tool call request
        "tools/call" => {
            let params = req.params.unwrap_or(Value::Null);
            let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let arguments = params.get("arguments").cloned().unwrap_or(Value::Object(serde_json::Map::new()));

            match handle_call_tool(vault, name, arguments).await {
                Ok(res) => Some(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: Some(id),
                    result: Some(res),
                    error: None,
                }),
                Err(e) => Some(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: Some(id),
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32603, // Internal Error
                        message: format!("Internal execution error: {}", e),
                        data: None,
                    }),
                }),
            }
        }
        // Catch-all for unrecognized JSON-RPC methods
        _ => {
            Some(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: Some(id),
                result: None,
                error: Some(JsonRpcError {
                    code: -32601, // Method Not Found
                    message: format!("Method not found: {}", req.method),
                    data: None,
                }),
            })
        }
    }
}

// ============================================================================
// 3. Entrypoint
// ============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    // 1. Resolve Vault Path: Checks variable first, then CLI fallback
    let vault_path = match env::var("OBSIDIAN_VAULT_PATH") {
        Ok(path) => path,
        Err(_) => {
            let args: Vec<String> = env::args().collect();
            if args.len() > 1 {
                args[1].clone()
            } else {
                eprintln!("[Obsidian MCP] Error: Target vault path not specified.");
                eprintln!("[Obsidian MCP] Set the OBSIDIAN_VAULT_PATH env variable or pass the path as a CLI argument.");
                std::process::exit(1);
            }
        }
    };

    // Standard Protocol Guard: We write all diagnostics strictly to stderr to prevent connection corruption
    eprintln!("[Obsidian MCP] Starting server...");
    eprintln!("[Obsidian MCP] Target vault: {}", vault_path);

    // Initialize Vault and check folder bounds
    let vault = match ObsidianVault::new(&vault_path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("[Obsidian MCP] Fatal error initializing vault: {}", e);
            std::process::exit(1);
        }
    };

    eprintln!("[Obsidian MCP] Vault initialized. Listening on stdin...");

    // Stdio processing loops
    let mut reader = BufReader::new(stdin()).lines();
    let mut stdout = tokio::io::stdout();

    while let Ok(Some(line)) = reader.next_line().await {
        if let Some(resp) = process_line(&line, &vault).await {
            if let Ok(resp_str) = serde_json::to_string(&resp) {
                let mut bytes = resp_str.into_bytes();
                bytes.push(b'\n');
                let _ = stdout.write_all(&bytes).await;
                let _ = stdout.flush().await;
            }
        }
    }

    eprintln!("[Obsidian MCP] Stdin channel closed. Shutting down.");
    Ok(())
}
