//! Deterministic stdio LSP fixture used by the Editor process tests.
//! This is intentionally tiny and has no shell or installed-tool dependency.

use std::io::{self, Read, Write};
use std::time::Duration;

use serde_json::{json, Value};

fn main() -> io::Result<()> {
    let arguments = std::env::args().collect::<Vec<_>>();
    let minimal = arguments.iter().any(|argument| argument == "minimal");
    let no_range_formatting = arguments
        .iter()
        .any(|argument| argument == "no-range-formatting");
    let no_organize_imports = arguments
        .iter()
        .any(|argument| argument == "no-organize-imports");
    let slow = arguments.iter().any(|argument| argument == "slow");
    let malformed = arguments.iter().any(|argument| argument == "malformed");
    let crash_after_open = arguments
        .iter()
        .any(|argument| argument == "crash-after-open");
    let stdin = io::stdin();
    let mut stdin = stdin.lock();
    while let Some(body) = read_frame(&mut stdin)? {
        let message: Value = match serde_json::from_slice(&body) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let method = message
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or_default();
        match method {
            "initialize" => write_message(&json!({
                "jsonrpc": "2.0",
                "id": message.get("id").cloned().unwrap_or(Value::Null),
                "result": { "capabilities": {
                    "completionProvider": if !minimal { json!({}) } else { Value::Null },
                    "signatureHelpProvider": if !minimal { json!({}) } else { Value::Null },
                    "hoverProvider": !minimal,
                    "definitionProvider": !minimal,
                    "referencesProvider": !minimal,
                    "renameProvider": !minimal,
                    "codeActionProvider": if !minimal && !no_organize_imports { json!(true) } else if !minimal { json!({ "codeActionKinds": ["quickfix"] }) } else { Value::Null },
                    "semanticTokensProvider": if !minimal { json!({ "legend": { "tokenTypes": ["function"] }, "full": true }) } else { Value::Null },
                    "documentSymbolProvider": !minimal,
                    "foldingRangeProvider": !minimal,
                    "documentFormattingProvider": !minimal,
                    "documentRangeFormattingProvider": !minimal && !no_range_formatting,
                }},
            }))?,
            "textDocument/didOpen" | "textDocument/didChange" => {
                if malformed {
                    write_raw(b"Content-Length: 4\r\n\r\nnope")?;
                }
                let document = message
                    .get("params")
                    .and_then(|params| params.get("textDocument"));
                let uri = document
                    .and_then(|document| document.get("uri"))
                    .cloned()
                    .unwrap_or(Value::Null);
                let version = document
                    .and_then(|document| document.get("version"))
                    .cloned()
                    .unwrap_or(json!(1));
                write_message(&json!({
                    "jsonrpc": "2.0", "method": "textDocument/publishDiagnostics",
                    "params": { "uri": uri, "version": version, "diagnostics": [{
                        "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 1 } },
                        "severity": 2, "message": "fixture diagnostic", "source": "fixture"
                    }] }
                }))?;
                if crash_after_open {
                    return Ok(());
                }
            }
            "shutdown" => write_message(
                &json!({ "jsonrpc": "2.0", "id": message.get("id"), "result": null }),
            )?,
            "exit" => return Ok(()),
            "textDocument/completion" => write_message(
                &json!({ "jsonrpc": "2.0", "id": message.get("id"), "result": { "isIncomplete": false, "items": [{ "label": "fixture", "kind": 3 }] } }),
            )?,
            "textDocument/signatureHelp" => write_message(
                &json!({ "jsonrpc": "2.0", "id": message.get("id"), "result": { "signatures": [{ "label": "fixture()", "documentation": { "kind": "markdown", "value": "fixture docs" } }] } }),
            )?,
            "textDocument/hover" => {
                if slow {
                    std::thread::sleep(Duration::from_millis(250));
                }
                write_message(
                    &json!({ "jsonrpc": "2.0", "id": message.get("id"), "result": { "contents": { "kind": "markdown", "value": "fixture hover" } } }),
                )?;
            }
            "textDocument/definition" | "textDocument/references" => write_message(
                &json!({ "jsonrpc": "2.0", "id": message.get("id"), "result": [{ "uri": "file:///fixture", "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 1 } } }] }),
            )?,
            "textDocument/rename" => write_message(
                &json!({ "jsonrpc": "2.0", "id": message.get("id"), "result": { "changes": { "file:///fixture": [{ "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 1 } }, "newText": "renamed" }] } } }),
            )?,
            "textDocument/codeAction" => write_message(
                &json!({ "jsonrpc": "2.0", "id": message.get("id"), "result": [{ "title": "fixture action", "kind": "quickfix", "edit": { "changes": { "file:///fixture": [{ "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 1 } }, "newText": "organized" }] } } }] }),
            )?,
            "textDocument/semanticTokens/full" => write_message(
                &json!({ "jsonrpc": "2.0", "id": message.get("id"), "result": { "data": [0, 0, 1, 0, 0] } }),
            )?,
            "textDocument/documentSymbol" => write_message(
                &json!({ "jsonrpc": "2.0", "id": message.get("id"), "result": [{ "name": "fixture", "kind": 12, "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 1 } }, "selectionRange": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 1 } } }] }),
            )?,
            "textDocument/foldingRange" => write_message(
                &json!({ "jsonrpc": "2.0", "id": message.get("id"), "result": [{ "startLine": 0, "endLine": 1, "kind": "region" }] }),
            )?,
            "textDocument/formatting" => write_message(
                &json!({ "jsonrpc": "2.0", "id": message.get("id"), "result": [{ "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 1 } }, "newText": "f" }] }),
            )?,
            "textDocument/rangeFormatting" => write_message(
                &json!({ "jsonrpc": "2.0", "id": message.get("id"), "result": [{ "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 1 } }, "newText": "range" }] }),
            )?,
            _ if message.get("id").is_some() => write_message(
                &json!({ "jsonrpc": "2.0", "id": message.get("id"), "error": { "code": -32601, "message": "unsupported" } }),
            )?,
            _ => {}
        }
    }
    Ok(())
}

fn read_frame(input: &mut impl Read) -> io::Result<Option<Vec<u8>>> {
    let mut header = Vec::new();
    let mut byte = [0_u8; 1];
    loop {
        if input.read(&mut byte)? == 0 {
            return Ok(None);
        }
        header.push(byte[0]);
        if header.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    let length = header
        .split(|byte| *byte == b'\n')
        .find_map(|line| {
            let colon = line.iter().position(|byte| *byte == b':')?;
            if line[..colon].eq_ignore_ascii_case(b"content-length") {
                std::str::from_utf8(&line[colon + 1..])
                    .ok()?
                    .trim()
                    .parse()
                    .ok()
            } else {
                None
            }
        })
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing Content-Length"))?;
    let mut body = vec![0_u8; length];
    input.read_exact(&mut body)?;
    Ok(Some(body))
}

fn write_message(message: &Value) -> io::Result<()> {
    let body = serde_json::to_vec(message).map_err(io::Error::other)?;
    write_raw_body(&body)
}

fn write_raw(body: &[u8]) -> io::Result<()> {
    write_raw_body(body)
}

fn write_raw_body(body: &[u8]) -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    write!(stdout, "Content-Length: {}\r\n\r\n", body.len())?;
    stdout.write_all(body)?;
    stdout.flush()
}
