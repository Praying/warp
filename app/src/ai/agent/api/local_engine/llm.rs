//! LLM client layer for the local agent engine: chat-history types, tool
//! (de)serialization, and streaming against OpenAI-compatible or
//! Anthropic-compatible endpoints.

use futures::StreamExt as _;
use futures::stream::{BoxStream, Stream};
use warp_multi_agent_api as api;
use warp_multi_agent_api::message::tool_call::{
    ApplyFileDiffs, FileGlobV2, Grep, ReadFiles, RunShellCommand, Tool,
};
use warp_multi_agent_api::run_shell_command_result::Result as RunResult;

macro_rules! flatten_tool_result {
    ($result:expr, $variant:ident) => {
        match $result.result.as_ref() {
            Some($variant::RunShellCommand(result)) => run_shell_result_text(result),
            Some($variant::ReadFiles(result)) => read_files_result_text(result),
            Some($variant::Grep(result)) => grep_result_text(result),
            Some($variant::FileGlobV2(result)) => file_glob_v2_result_text(result),
            Some($variant::ApplyFileDiffs(result)) => apply_diffs_result_text(result),
            _ => "(tool result)".to_owned(),
        }
    };
}

/// The endpoint the local engine calls.
#[derive(Clone)]
pub(super) struct LlmModel {
    pub base_url: String,
    pub api_key: String,
    pub schema: api::request::settings::custom_model_providers::CustomEndpointSchema,
    pub model_id: String,
    pub reasoning_effort: Option<String>,
}

pub(super) struct StreamingRequest {
    pub model: LlmModel,
    pub system_prompt: String,
    pub messages: Vec<ChatMessage>,
    pub max_output_tokens: u32,
}

pub(super) enum ChatMessage {
    User(String),
    Assistant(String),
    AssistantToolCalls(Vec<ToolCallRequest>),
    ToolResult { id: String, content: String },
}

impl ChatMessage {
    pub(super) fn user(content: impl Into<String>) -> Self {
        Self::User(content.into())
    }

    pub(super) fn assistant(content: impl Into<String>) -> Self {
        Self::Assistant(content.into())
    }

    pub(super) fn assistant_tool_calls(tool_calls: Vec<ToolCallRequest>) -> Self {
        Self::AssistantToolCalls(tool_calls)
    }

    pub(super) fn tool_result(id: impl Into<String>, content: impl Into<String>) -> Self {
        Self::ToolResult {
            id: id.into(),
            content: content.into(),
        }
    }
}

pub(super) struct ToolCallRequest {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Map<String, serde_json::Value>,
}

pub(super) enum StreamChunk {
    Text(String),
    ToolCall(ToolCallRequest),
}

impl ToolCallRequest {
    /// Rebuilds the request-shape arguments of a tool call from the message
    /// history, so the model sees its own past calls in its native format.
    pub(super) fn from_proto(tool_call: &api::message::ToolCall) -> Option<Self> {
        let tool = tool_call.tool.as_ref()?;
        let mut arguments = serde_json::Map::new();
        let name = match tool {
            Tool::RunShellCommand(RunShellCommand { command, .. }) => {
                arguments.insert(
                    "command".to_owned(),
                    serde_json::Value::String(command.clone()),
                );
                "run_shell_command"
            }
            Tool::ReadFiles(ReadFiles { files, .. }) => {
                arguments.insert(
                    "files".to_owned(),
                    serde_json::Value::Array(
                        files
                            .iter()
                            .map(|file| serde_json::Value::String(file.name.clone()))
                            .collect(),
                    ),
                );
                "read_files"
            }
            Tool::Grep(Grep { queries, path }) => {
                arguments.insert(
                    "queries".to_owned(),
                    serde_json::Value::Array(
                        queries
                            .iter()
                            .map(|query| serde_json::Value::String(query.clone()))
                            .collect(),
                    ),
                );
                arguments.insert("path".to_owned(), serde_json::Value::String(path.clone()));
                "grep"
            }
            Tool::FileGlobV2(FileGlobV2 {
                patterns,
                search_dir,
                ..
            }) => {
                arguments.insert(
                    "patterns".to_owned(),
                    serde_json::Value::Array(
                        patterns
                            .iter()
                            .map(|pattern| serde_json::Value::String(pattern.clone()))
                            .collect(),
                    ),
                );
                arguments.insert(
                    "search_dir".to_owned(),
                    serde_json::Value::String(search_dir.clone()),
                );
                "file_glob"
            }
            Tool::ApplyFileDiffs(ApplyFileDiffs {
                summary,
                diffs,
                new_files,
                ..
            }) => {
                arguments.insert(
                    "summary".to_owned(),
                    serde_json::Value::String(summary.clone()),
                );
                arguments.insert(
                    "diffs".to_owned(),
                    serde_json::Value::Array(
                        diffs
                            .iter()
                            .map(|diff| {
                                serde_json::json!({
                                    "file_path": diff.file_path,
                                    "search": diff.search,
                                    "replace": diff.replace,
                                })
                            })
                            .collect(),
                    ),
                );
                arguments.insert(
                    "new_files".to_owned(),
                    serde_json::Value::Array(
                        new_files
                            .iter()
                            .map(|file| {
                                serde_json::json!({
                                    "file_path": file.file_path,
                                    "content": file.content,
                                })
                            })
                            .collect(),
                    ),
                );
                "apply_file_diffs"
            }
            // Server-side or unsupported tools carry no request-shape
            // representation for the model.
            _ => return None,
        };
        Some(Self {
            id: tool_call.tool_call_id.clone(),
            name: name.to_owned(),
            arguments,
        })
    }
}

/// Flattens a tool result from the task history into text for the model.
pub(super) fn tool_result_text(result: &api::message::ToolCallResult) -> String {
    use api::message::tool_call_result::Result as R;
    match result.result.as_ref() {
        Some(R::Cancel(_)) => "(cancelled by the user)".to_owned(),
        _ => flatten_tool_result!(result, R),
    }
}

/// Flattens a tool result from the request input into text for the model.
/// The input carries a distinct `ToolCallResult` type with the same variants.
pub(super) fn input_tool_result_text(result: &api::request::input::ToolCallResult) -> String {
    use api::request::input::tool_call_result::Result as R;
    flatten_tool_result!(result, R)
}

fn run_shell_result_text(result: &api::RunShellCommandResult) -> String {
    let (output, exit_code) = match result.result.as_ref() {
        Some(RunResult::CommandFinished(finished)) => (finished.output.clone(), finished.exit_code),
        _ =>
        {
            #[allow(deprecated)]
            (result.output.clone(), result.exit_code)
        }
    };
    format!("(exit code {exit_code})\n{output}")
}

fn read_files_result_text(result: &api::ReadFilesResult) -> String {
    match result.result.as_ref() {
        Some(api::read_files_result::Result::TextFilesSuccess(success)) => success
            .files
            .iter()
            .map(|file| format!("--- {} ---\n{}", file.file_path, file.content))
            .collect::<Vec<_>>()
            .join("\n\n"),
        Some(api::read_files_result::Result::Error(error)) => {
            format!("error: {}", error.message)
        }
        _ => "(files read)".to_owned(),
    }
}

fn grep_result_text(result: &api::GrepResult) -> String {
    match result.result.as_ref() {
        Some(api::grep_result::Result::Success(success)) => success
            .matched_files
            .iter()
            .map(|file| {
                let lines = file
                    .matched_lines
                    .iter()
                    .map(|line| line.line_number.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{} (lines: {lines})", file.file_path)
            })
            .collect::<Vec<_>>()
            .join("\n"),
        Some(api::grep_result::Result::Error(error)) => {
            format!("error: {}", error.message)
        }
        None => "(grep result)".to_owned(),
    }
}

fn file_glob_v2_result_text(result: &api::FileGlobV2Result) -> String {
    match result.result.as_ref() {
        Some(api::file_glob_v2_result::Result::Success(success)) => success
            .matched_files
            .iter()
            .map(|file| file.file_path.clone())
            .collect::<Vec<_>>()
            .join("\n"),
        Some(api::file_glob_v2_result::Result::Error(error)) => {
            format!("error: {}", error.message)
        }
        None => "(file glob result)".to_owned(),
    }
}

fn apply_diffs_result_text(result: &api::ApplyFileDiffsResult) -> String {
    match result.result.as_ref() {
        Some(api::apply_file_diffs_result::Result::Success(_)) => "diffs applied".to_owned(),
        Some(api::apply_file_diffs_result::Result::Error(error)) => {
            format!("error: {}", error.message)
        }
        None => "(diff result)".to_owned(),
    }
}

/// Streams a completion from the configured endpoint. Tool-call fragments are
/// accumulated and emitted as complete calls.
pub(super) fn stream_completion(
    request: StreamingRequest,
) -> BoxStream<'static, Result<StreamChunk, String>> {
    use api::request::settings::custom_model_providers::CustomEndpointSchema;
    match request.model.schema {
        CustomEndpointSchema::OpenaiChatCompletions => openai::stream(request),
        CustomEndpointSchema::AnthropicMessages => anthropic::stream(request),
        CustomEndpointSchema::OpenaiResponses => futures::stream::once(async {
            Err(
                "the OpenAI Responses schema is not supported by the local agent; \
                     configure the endpoint as OpenAI Chat Completions or Anthropic Messages"
                    .to_owned(),
            )
        })
        .boxed(),
    }
}

fn http_client() -> &'static http_client::Client {
    use std::sync::OnceLock;
    static CLIENT: OnceLock<http_client::Client> = OnceLock::new();
    CLIENT.get_or_init(http_client::Client::new_external)
}

/// The tool catalog offered to the model, as OpenAI-style function specs.
fn tool_specs() -> serde_json::Value {
    serde_json::json!([
        {
            "type": "function",
            "function": {
                "name": "run_shell_command",
                "description": "Run a shell command in the user's terminal. The user \
                    reviews the command before it runs. Returns the command output.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "The shell command to run",
                        },
                    },
                    "required": ["command"],
                },
            },
        },
        {
            "type": "function",
            "function": {
                "name": "read_files",
                "description": "Read the contents of files.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "files": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "The file paths to read",
                        },
                    },
                    "required": ["files"],
                },
            },
        },
        {
            "type": "function",
            "function": {
                "name": "grep",
                "description": "Search file contents for strings or regular expressions.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "queries": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "The search terms or patterns",
                        },
                        "path": {
                            "type": "string",
                            "description": "The relative path to the file or directory to search in",
                        },
                    },
                    "required": ["queries"],
                },
            },
        },
        {
            "type": "function",
            "function": {
                "name": "file_glob",
                "description": "Find files whose names match glob patterns (?, *, []).",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "patterns": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "The glob patterns to match",
                        },
                        "search_dir": {
                            "type": "string",
                            "description": "The relative path to the directory to search in",
                        },
                    },
                    "required": ["patterns"],
                },
            },
        },
        {
            "type": "function",
            "function": {
                "name": "apply_file_diffs",
                "description": "Apply edits to files. For each diff, the exact existing \
                    `search` text is replaced with `replace`. Use `new_files` to create files.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "summary": {
                            "type": "string",
                            "description": "A short description of the edits",
                        },
                        "diffs": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "file_path": { "type": "string" },
                                    "search": { "type": "string" },
                                    "replace": { "type": "string" },
                                },
                                "required": ["file_path", "search", "replace"],
                            },
                        },
                        "new_files": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "file_path": { "type": "string" },
                                    "content": { "type": "string" },
                                },
                                "required": ["file_path", "content"],
                            },
                        },
                    },
                },
            },
        },
    ])
}

struct EndpointError {
    status: Option<http::StatusCode>,
    message: String,
}

impl EndpointError {
    fn is_bad_request(&self) -> bool {
        self.status == Some(http::StatusCode::BAD_REQUEST)
    }

    fn message(&self) -> String {
        match self.status {
            Some(status) => format!("endpoint returned {status}: {}", self.message),
            None => self.message.clone(),
        }
    }
}

/// Shared SSE plumbing: POSTs a JSON body and yields the parsed `data`
/// payload of each event.
fn sse_data_stream(
    url: String,
    headers: Vec<(String, String)>,
    body: serde_json::Value,
) -> impl Stream<Item = Result<serde_json::Value, EndpointError>> {
    let client = http_client();
    let mut builder = client
        .post(url)
        .json(&body)
        .prevent_sleep("Local agent request in-progress");
    for (name, value) in headers {
        builder = builder.header(name, value);
    }
    builder.eventsource().filter_map(|event| async {
        match event {
            Ok(reqwest_eventsource::Event::Open) => None,
            Ok(reqwest_eventsource::Event::Message(message)) => {
                if message.data.trim() == "[DONE]" {
                    None
                } else {
                    Some(
                        serde_json::from_str(&message.data).map_err(|error| EndpointError {
                            status: None,
                            message: format!("failed to parse stream event: {error}"),
                        }),
                    )
                }
            }
            Err(reqwest_eventsource::Error::InvalidStatusCode(status, response)) => {
                let message = match response.bytes().await {
                    Ok(bytes) => String::from_utf8_lossy(&bytes).to_string(),
                    Err(error) => format!("failed to read error body: {error}"),
                };
                Some(Err(EndpointError {
                    status: Some(status),
                    message,
                }))
            }
            Err(error) => Some(Err(EndpointError {
                status: None,
                message: error.to_string(),
            })),
        }
    })
}

/// Guards against an endpoint that rejects the tool catalog (common for
/// minimal OpenAI-compatible servers): retries once without tools on a 400.
type RetryFactory =
    Box<dyn FnOnce() -> BoxStream<'static, Result<StreamChunk, String>> + Send + 'static>;

fn with_toolless_retry(
    first: BoxStream<'static, Result<StreamChunk, EndpointError>>,
    retry: RetryFactory,
) -> BoxStream<'static, Result<StreamChunk, String>> {
    enum State {
        First(
            BoxStream<'static, Result<StreamChunk, EndpointError>>,
            Option<RetryFactory>,
        ),
        Second(BoxStream<'static, Result<StreamChunk, String>>),
    }
    futures::stream::unfold(State::First(first, Some(retry)), move |state| async move {
        match state {
            State::First(mut stream, retry) => match stream.next().await {
                Some(Ok(chunk)) => Some((Ok(chunk), State::First(stream, retry))),
                Some(Err(error)) => {
                    if error.is_bad_request()
                        && let Some(retry) = retry
                    {
                        log::warn!(
                            "Local agent request rejected with the tool catalog; \
                             retrying without tools: {}",
                            error.message()
                        );
                        let mut second = retry();
                        second
                            .next()
                            .await
                            .map(|item| (item, State::Second(second)))
                    } else {
                        Some((Err(error.message()), State::First(stream, None)))
                    }
                }
                None => None,
            },
            State::Second(mut stream) => stream
                .next()
                .await
                .map(|item| (item, State::Second(stream))),
        }
    })
    .boxed()
}

/// Flattens a stream of grouped chunk results into single chunks, mapping
/// errors along the way.
fn flatten_chunk_results(
    stream: impl Stream<Item = Result<Vec<StreamChunk>, EndpointError>> + Send + 'static,
) -> BoxStream<'static, Result<StreamChunk, EndpointError>> {
    stream
        .flat_map(|result| {
            futures::stream::iter(match result {
                Ok(chunks) => chunks.into_iter().map(Ok).collect(),
                Err(error) => vec![Err(error)],
            })
        })
        .boxed()
}

mod openai {
    use super::*;

    pub(super) fn stream(
        request: StreamingRequest,
    ) -> BoxStream<'static, Result<StreamChunk, String>> {
        let url = format!(
            "{}/chat/completions",
            request.model.base_url.trim_end_matches('/')
        );
        let mut headers = Vec::new();
        if !request.model.api_key.is_empty() {
            headers.push((
                "Authorization".to_owned(),
                format!("Bearer {}", request.model.api_key),
            ));
        }
        let mut body = serde_json::json!({
            "model": request.model.model_id,
            "stream": true,
            "messages": openai_messages(&request),
        });
        if let Some(reasoning_effort) = &request.model.reasoning_effort {
            body["reasoning_effort"] = serde_json::Value::String(reasoning_effort.clone());
        }

        let mut body_with_tools = body.clone();
        body_with_tools["tools"] = tool_specs();
        let first = chunk_stream(url.clone(), headers.clone(), body_with_tools);
        with_toolless_retry(
            first,
            Box::new(move || {
                chunk_stream(url, headers, body)
                    .map(|result| result.map_err(|error| error.message()))
                    .boxed()
            }),
        )
    }

    fn chunk_stream(
        url: String,
        headers: Vec<(String, String)>,
        body: serde_json::Value,
    ) -> BoxStream<'static, Result<StreamChunk, EndpointError>> {
        flatten_chunk_results(sse_data_stream(url, headers, body).scan(
            Accumulator::default(),
            |accumulator, result| {
                futures::future::ready(Some(match result {
                    Ok(payload) => Ok(accumulator.map_payload(&payload)),
                    Err(error) => Err(error),
                }))
            },
        ))
    }

    /// Accumulates streamed tool-call fragments, keyed by their index.
    #[derive(Default)]
    pub(super) struct Accumulator {
        tool_calls: Vec<PendingToolCall>,
    }

    #[derive(Default)]
    struct PendingToolCall {
        id: String,
        name: String,
        arguments: String,
    }

    impl Accumulator {
        pub(super) fn map_payload(&mut self, payload: &serde_json::Value) -> Vec<StreamChunk> {
            let mut chunks = Vec::new();
            let Some(choice) = payload
                .get("choices")
                .and_then(serde_json::Value::as_array)
                .and_then(|choices| choices.first())
            else {
                return chunks;
            };
            if let Some(content) = choice
                .pointer("/delta/content")
                .and_then(serde_json::Value::as_str)
                && !content.is_empty()
            {
                chunks.push(StreamChunk::Text(content.to_owned()));
            }
            if let Some(fragments) = choice
                .pointer("/delta/tool_calls")
                .and_then(serde_json::Value::as_array)
            {
                for fragment in fragments {
                    let index = fragment
                        .get("index")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or_default() as usize;
                    while self.tool_calls.len() <= index {
                        self.tool_calls.push(PendingToolCall::default());
                    }
                    let pending = &mut self.tool_calls[index];
                    if let Some(id) = fragment.get("id").and_then(serde_json::Value::as_str) {
                        pending.id = id.to_owned();
                    }
                    if let Some(name) = fragment
                        .pointer("/function/name")
                        .and_then(serde_json::Value::as_str)
                    {
                        pending.name = name.to_owned();
                    }
                    if let Some(arguments) = fragment
                        .pointer("/function/arguments")
                        .and_then(serde_json::Value::as_str)
                    {
                        pending.arguments.push_str(arguments);
                    }
                }
            }
            if choice
                .get("finish_reason")
                .and_then(serde_json::Value::as_str)
                == Some("tool_calls")
            {
                chunks.extend(self.flush_tool_calls());
            }
            chunks
        }

        fn flush_tool_calls(&mut self) -> Vec<StreamChunk> {
            self.tool_calls
                .drain(..)
                .filter(|pending| !pending.name.is_empty())
                .map(|pending| {
                    StreamChunk::ToolCall(ToolCallRequest {
                        id: if pending.id.is_empty() {
                            uuid::Uuid::new_v4().to_string()
                        } else {
                            pending.id
                        },
                        name: pending.name,
                        arguments: serde_json::from_str(&pending.arguments).unwrap_or_default(),
                    })
                })
                .collect()
        }
    }

    fn openai_messages(request: &StreamingRequest) -> Vec<serde_json::Value> {
        let mut messages = vec![serde_json::json!({
            "role": "system",
            "content": request.system_prompt,
        })];
        for message in &request.messages {
            match message {
                ChatMessage::User(content) => messages.push(serde_json::json!({
                    "role": "user",
                    "content": content,
                })),
                ChatMessage::Assistant(content) => messages.push(serde_json::json!({
                    "role": "assistant",
                    "content": content,
                })),
                ChatMessage::AssistantToolCalls(tool_calls) => messages.push(serde_json::json!({
                    "role": "assistant",
                    "content": "",
                    "tool_calls": tool_calls
                        .iter()
                        .map(openai_tool_call)
                        .collect::<Vec<_>>(),
                })),
                ChatMessage::ToolResult { id, content } => messages.push(serde_json::json!({
                    "role": "tool",
                    "tool_call_id": id,
                    "content": content,
                })),
            }
        }
        messages
    }

    fn openai_tool_call(tool_call: &ToolCallRequest) -> serde_json::Value {
        serde_json::json!({
            "id": tool_call.id,
            "type": "function",
            "function": {
                "name": tool_call.name,
                "arguments": serde_json::Value::String(
                    serde_json::to_string(&tool_call.arguments).unwrap_or_default(),
                ),
            },
        })
    }
}

mod anthropic {
    use super::*;

    pub(super) fn stream(
        request: StreamingRequest,
    ) -> BoxStream<'static, Result<StreamChunk, String>> {
        let url = format!("{}/messages", request.model.base_url.trim_end_matches('/'));
        let headers = vec![
            ("x-api-key".to_owned(), request.model.api_key.clone()),
            ("anthropic-version".to_owned(), "2023-06-01".to_owned()),
        ];
        let body = serde_json::json!({
            "model": request.model.model_id,
            "system": request.system_prompt,
            "stream": true,
            "max_tokens": request.max_output_tokens,
            "messages": anthropic_messages(&request.messages),
            "tools": anthropic_tools(),
        });
        flatten_chunk_results(sse_data_stream(url, headers, body).scan(
            Accumulator::default(),
            |accumulator, result| {
                futures::future::ready(Some(match result {
                    Ok(payload) => Ok(accumulator.map_payload(&payload)),
                    Err(error) => Err(error),
                }))
            },
        ))
        .map(|result| result.map_err(|error| error.message()))
        .boxed()
    }

    fn anthropic_tools() -> serde_json::Value {
        let specs = tool_specs().as_array().cloned().unwrap_or_default();
        specs
            .iter()
            .filter_map(|spec| {
                let function = spec.get("function")?;
                Some(serde_json::json!({
                    "name": function.get("name")?,
                    "description": function.get("description")?,
                    "input_schema": function.get("parameters")?,
                }))
            })
            .collect::<Vec<_>>()
            .into()
    }

    fn anthropic_messages(messages: &[ChatMessage]) -> Vec<serde_json::Value> {
        messages
            .iter()
            .map(|message| match message {
                ChatMessage::User(content) => serde_json::json!({
                    "role": "user",
                    "content": content,
                }),
                ChatMessage::Assistant(content) => serde_json::json!({
                    "role": "assistant",
                    "content": content,
                }),
                ChatMessage::AssistantToolCalls(tool_calls) => serde_json::json!({
                    "role": "assistant",
                    "content": tool_calls.iter().map(|tool_call| serde_json::json!({
                        "type": "tool_use",
                        "id": tool_call.id,
                        "name": tool_call.name,
                        "input": tool_call.arguments,
                    })).collect::<Vec<_>>(),
                }),
                ChatMessage::ToolResult { id, content } => serde_json::json!({
                    "role": "user",
                    "content": [{
                        "type": "tool_result",
                        "tool_use_id": id,
                        "content": content,
                    }],
                }),
            })
            .collect()
    }

    /// Tracks the currently open `tool_use` content block, whose input JSON
    /// streams as `input_json_delta` fragments until the block stops.
    #[derive(Default)]
    pub(super) struct Accumulator {
        open_tool: Option<PendingToolUse>,
    }

    #[derive(Default)]
    struct PendingToolUse {
        id: String,
        name: String,
        json: String,
    }

    impl Accumulator {
        pub(super) fn map_payload(&mut self, payload: &serde_json::Value) -> Vec<StreamChunk> {
            let mut chunks = Vec::new();
            match payload.get("type").and_then(serde_json::Value::as_str) {
                Some("content_block_start") => {
                    let block = payload.get("content_block");
                    if block
                        .and_then(|block| block.get("type"))
                        .and_then(serde_json::Value::as_str)
                        == Some("tool_use")
                    {
                        self.open_tool = Some(PendingToolUse {
                            id: block
                                .and_then(|block| block.get("id"))
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or_default()
                                .to_owned(),
                            name: block
                                .and_then(|block| block.get("name"))
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or_default()
                                .to_owned(),
                            json: String::new(),
                        });
                    }
                }
                Some("content_block_delta") => {
                    if let Some(text) = payload
                        .pointer("/delta/text")
                        .and_then(serde_json::Value::as_str)
                        && !text.is_empty()
                    {
                        chunks.push(StreamChunk::Text(text.to_owned()));
                    }
                    if let Some(partial_json) = payload
                        .pointer("/delta/partial_json")
                        .and_then(serde_json::Value::as_str)
                        && let Some(tool) = self.open_tool.as_mut()
                    {
                        tool.json.push_str(partial_json);
                    }
                }
                Some("content_block_stop") => {
                    if let Some(tool) = self.open_tool.take()
                        && !tool.name.is_empty()
                    {
                        chunks.push(StreamChunk::ToolCall(ToolCallRequest {
                            id: tool.id,
                            name: tool.name,
                            arguments: serde_json::from_str(&tool.json).unwrap_or_default(),
                        }));
                    }
                }
                _ => {}
            }
            chunks
        }
    }
}

#[cfg(test)]
#[path = "llm_tests.rs"]
mod tests;
