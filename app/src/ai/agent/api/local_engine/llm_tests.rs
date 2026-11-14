use super::*;

fn text_chunk(chunk: &StreamChunk) -> &str {
    match chunk {
        StreamChunk::Text(text) => text,
        StreamChunk::ToolCall(_) | StreamChunk::Usage { .. } => panic!("expected a text chunk"),
    }
}

fn tool_chunk(chunk: &StreamChunk) -> &ToolCallRequest {
    match chunk {
        StreamChunk::ToolCall(tool) => tool,
        StreamChunk::Text(_) | StreamChunk::Usage { .. } => panic!("expected a tool call chunk"),
    }
}

#[test]
fn openai_usage_only_chunk_preserves_zero_and_does_not_double_count_cached_tokens() {
    let mut accumulator = openai::Accumulator::default();
    let chunks = accumulator.map_payload(&serde_json::json!({
        "choices": [],
        "usage": {
            "prompt_tokens": 120,
            "completion_tokens": 0,
            "prompt_tokens_details": { "cached_tokens": 100 },
        },
    }));
    assert!(matches!(
        chunks.as_slice(),
        [StreamChunk::Usage {
            input_tokens: Some(120),
            output_tokens: Some(0),
        }]
    ));
    assert!(
        accumulator
            .map_payload(&serde_json::json!({"usage": null, "choices": []}))
            .is_empty()
    );
}

#[test]
fn anthropic_usage_includes_cache_and_preserves_missing_input_on_delta() {
    let mut accumulator = anthropic::Accumulator::default();
    let start = accumulator.map_payload(&serde_json::json!({
        "type": "message_start",
        "message": { "usage": {
            "input_tokens": 20,
            "cache_creation_input_tokens": 40,
            "cache_read_input_tokens": 60,
            "output_tokens": 1,
        }},
    }));
    assert!(matches!(
        start.as_slice(),
        [StreamChunk::Usage {
            input_tokens: Some(120),
            output_tokens: Some(1),
        }]
    ));
    let delta = accumulator.map_payload(&serde_json::json!({
        "type": "message_delta", "usage": { "output_tokens": 30 },
    }));
    assert!(matches!(
        delta.as_slice(),
        [StreamChunk::Usage {
            input_tokens: None,
            output_tokens: Some(30),
        }]
    ));
}

#[test]
fn openai_retries_without_usage_options_but_keeps_tools() {
    let mut server = mockito::Server::new();
    let rejected = server
        .mock("POST", "/chat/completions")
        .match_body(mockito::Matcher::PartialJson(serde_json::json!({
            "stream_options": {"include_usage": true},
        })))
        .with_status(400)
        .with_body("unsupported stream_options")
        .create();
    let accepted = server
        .mock("POST", "/chat/completions")
        .match_request(|request| {
            let body: serde_json::Value = serde_json::from_slice(request.body().unwrap()).unwrap();
            body.get("stream_options").is_none() && body.get("tools").is_some()
        })
        .with_header("content-type", "text/event-stream")
        .with_body("data: {\"choices\":[{\"delta\":{\"content\":\"ok\"}}]}\n\ndata: [DONE]\n\n")
        .create();
    let request = StreamingRequest {
        model: LlmModel {
            base_url: server.url(),
            ..llm_model()
        },
        system_prompt: "test".to_owned(),
        messages: vec![ChatMessage::user("hello")],
    };
    let chunks = futures::executor::block_on(openai::stream(request).collect::<Vec<_>>());
    assert_eq!(chunks.len(), 1);
    assert_eq!(text_chunk(chunks[0].as_ref().unwrap()), "ok");
    rejected.assert();
    accepted.assert();
}

#[test]
fn openai_accumulator_streams_text_and_flushes_tool_calls() {
    let mut accumulator = openai::Accumulator::default();
    let text = accumulator.map_payload(&serde_json::json!({
        "choices": [{"delta": {"content": "hello"}}],
    }));
    assert_eq!(text_chunk(&text[0]), "hello");

    accumulator.map_payload(&serde_json::json!({
        "choices": [{"delta": {"tool_calls": [
            {"index": 0, "id": "call-1", "function": {"name": "run_shell_command", "arguments": "{\"comm"}},
        ]}}],
    }));
    let flushed = accumulator.map_payload(&serde_json::json!({
        "choices": [{"delta": {"tool_calls": [
            {"index": 0, "function": {"arguments": "and\":\"ls\"}"}},
        ]}, "finish_reason": "tool_calls"}],
    }));
    assert!(flushed.len() == 1);
    let tool = tool_chunk(&flushed[0]);
    assert_eq!(tool.id, "call-1");
    assert_eq!(tool.name, "run_shell_command");
    assert_eq!(tool.arguments.get("command").unwrap(), "ls");
}

#[test]
fn anthropic_accumulator_buffers_tool_use_input() {
    let mut accumulator = anthropic::Accumulator::default();

    accumulator.map_payload(&serde_json::json!({
        "type": "content_block_start",
        "content_block": {"type": "tool_use", "id": "toolu-1", "name": "read_files"},
    }));
    let text = accumulator.map_payload(&serde_json::json!({
        "type": "content_block_delta",
        "delta": {"type": "input_json_delta", "partial_json": "{\"files\":"},
    }));
    assert!(text.is_empty());
    accumulator.map_payload(&serde_json::json!({
        "type": "content_block_delta",
        "delta": {"type": "input_json_delta", "partial_json": "[\"a.rs\"]}"},
    }));
    let flushed = accumulator.map_payload(&serde_json::json!({
        "type": "content_block_stop",
    }));
    assert!(flushed.len() == 1);
    let tool = tool_chunk(&flushed[0]);
    assert_eq!(tool.id, "toolu-1");
    assert_eq!(tool.name, "read_files");
    assert_eq!(
        tool.arguments.get("files").unwrap(),
        &serde_json::json!(["a.rs"])
    );
}

#[test]
fn tool_call_round_trips_through_proto() {
    let request = ToolCallRequest {
        id: "call-1".to_owned(),
        name: "run_shell_command".to_owned(),
        arguments: serde_json::json!({"command": "ls -la"})
            .as_object()
            .cloned()
            .unwrap(),
    };
    let proto = api::message::ToolCall {
        tool_call_id: "call-1".to_owned(),
        tool: Some(api::message::tool_call::Tool::RunShellCommand(
            api::message::tool_call::RunShellCommand {
                command: "ls -la".to_owned(),
                ..Default::default()
            },
        )),
    };
    let round_tripped = ToolCallRequest::from_proto(&proto).unwrap();
    assert_eq!(round_tripped.id, request.id);
    assert_eq!(round_tripped.name, request.name);
    assert_eq!(round_tripped.arguments, request.arguments);
}

#[test]
fn run_shell_result_text_reports_output_and_exit_code() {
    let result = api::RunShellCommandResult {
        result: Some(RunResult::CommandFinished(api::ShellCommandFinished {
            output: "total 0".to_owned(),
            exit_code: 0,
            ..Default::default()
        })),
        ..Default::default()
    };
    let message_result = api::message::ToolCallResult {
        tool_call_id: "call-1".to_owned(),
        result: Some(api::message::tool_call_result::Result::RunShellCommand(
            result,
        )),
        ..Default::default()
    };
    assert_eq!(tool_result_text(&message_result), "(exit code 0)\ntotal 0");
}

#[test]
fn anthropic_urls_pair_the_v1_suffix_both_ways() {
    assert_eq!(
        anthropic::messages_url("https://api.anthropic.com"),
        "https://api.anthropic.com/v1/messages"
    );
    assert_eq!(
        anthropic::messages_url("https://api.anthropic.com/v1"),
        "https://api.anthropic.com/v1/messages"
    );
    assert_eq!(
        anthropic::alternate_messages_url("https://api.anthropic.com"),
        "https://api.anthropic.com/messages"
    );
    assert_eq!(
        anthropic::alternate_messages_url("https://api.anthropic.com/v1/"),
        "https://api.anthropic.com/v1/v1/messages"
    );
}

#[test]
fn anthropic_messages_merge_consecutive_same_role_turns() {
    let messages = vec![
        ChatMessage::user("list files"),
        ChatMessage::assistant("Running a command."),
        ChatMessage::assistant_tool_calls(vec![ToolCallRequest {
            id: "call-1".to_owned(),
            name: "run_shell_command".to_owned(),
            arguments: serde_json::json!({"command": "ls"})
                .as_object()
                .cloned()
                .unwrap(),
        }]),
        ChatMessage::tool_result("call-1", "README.md"),
    ];
    let turns = anthropic::anthropic_messages(&messages);
    assert_eq!(turns.len(), 3);
    assert_eq!(turns[0]["role"], "user");
    assert_eq!(turns[1]["role"], "assistant");
    // Text and tool_use blocks share the assistant turn.
    let blocks = turns[1]["content"].as_array().unwrap();
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0]["type"], "text");
    assert_eq!(blocks[1]["type"], "tool_use");
    assert_eq!(turns[2]["role"], "user");
    assert_eq!(turns[2]["content"][0]["type"], "tool_result");
}

#[test]
fn gateway_not_found_bodies_are_recognized_and_cleaned() {
    let error = EndpointError {
        status: Some(http::StatusCode::BAD_REQUEST),
        message: clean_error_body(
            "data:{\"error\":{\"code\":\"400\",\"message\":\"Invalid request\",\"param\":\"404 NOT_FOUND\",\"type\":\"\"}}",
        ),
        url: "https://gw.example.com/v1/messages".to_owned(),
        model: "claude-3".to_owned(),
    };
    assert!(error.is_not_found());
    assert!(!error.message().starts_with("data:"));
    assert!(
        error
            .message()
            .contains("https://gw.example.com/v1/messages")
    );
    assert!(error.message().contains("claude-3"));
}

#[test]
fn thinking_budget_maps_effort_and_respects_max_tokens() {
    let model = |reasoning_effort: Option<&str>, max_output_tokens: Option<u32>| LlmModel {
        reasoning_effort: reasoning_effort.map(str::to_owned),
        max_output_tokens,
        ..llm_model()
    };
    assert_eq!(
        anthropic::thinking_budget(&model(Some("low"), None)),
        Some(1024)
    );
    assert_eq!(
        anthropic::thinking_budget(&model(Some("medium"), None)),
        Some(4096)
    );
    // Without a configured max output, the default 8192 caps the budget.
    assert_eq!(
        anthropic::thinking_budget(&model(Some("high"), None)),
        Some(7168)
    );
    assert_eq!(
        anthropic::thinking_budget(&model(Some("bogus"), None)),
        None
    );
    assert_eq!(anthropic::thinking_budget(&model(None, None)), None);
    // The budget must stay below max_tokens.
    assert_eq!(
        anthropic::thinking_budget(&model(Some("high"), Some(4096))),
        Some(3072)
    );
}

#[test]
fn openai_user_message_carries_images_as_content_blocks() {
    let request = StreamingRequest {
        model: llm_model(),
        system_prompt: "system".to_owned(),
        messages: vec![ChatMessage::user_with_images(
            "what is this?",
            vec![InputImage {
                data_base64: "aGVsbG8=".to_owned(),
                mime_type: "image/png".to_owned(),
            }],
        )],
    };
    let messages = openai::openai_messages(&request);
    let content = &messages[1]["content"];
    assert_eq!(content.as_array().unwrap().len(), 2);
    assert_eq!(content[0]["type"], "text");
    assert_eq!(content[1]["type"], "image_url");
    assert_eq!(
        content[1]["image_url"]["url"],
        "data:image/png;base64,aGVsbG8="
    );
}

#[test]
fn anthropic_user_message_carries_image_blocks() {
    let messages = vec![ChatMessage::user_with_images(
        "describe",
        vec![InputImage {
            data_base64: "aGVsbG8=".to_owned(),
            mime_type: "image/jpeg".to_owned(),
        }],
    )];
    let turns = anthropic::anthropic_messages(&messages);
    let blocks = turns[0]["content"].as_array().unwrap();
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[1]["type"], "image");
    assert_eq!(blocks[1]["source"]["media_type"], "image/jpeg");
}

fn llm_model() -> LlmModel {
    LlmModel {
        base_url: "http://localhost".to_owned(),
        api_key: String::new(),
        schema: api::request::settings::custom_model_providers::CustomEndpointSchema::OpenaiChatCompletions,
        model_id: "test".to_owned(),
        reasoning_effort: None,
        context_window: None,
        max_output_tokens: None,
        supports_images: false,
    }
}
