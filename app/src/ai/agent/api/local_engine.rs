//! Local agent engine for account-free sessions.
//!
//! Warp's agent loop normally runs on warp-server: the client sends the full
//! task history with each request and applies the server's `ClientAction`
//! stream. When the session has no server credentials, this module runs the
//! same loop locally against a user-configured model (custom endpoint or BYO
//! API key): it flattens the task history into chat messages, streams the
//! model's output as the same response events the server would send, and lets
//! the existing client-side tool execution (approvals, terminal runs) close
//! the loop by feeding results back through the next request.

use futures::StreamExt as _;
use futures::stream::BoxStream;
use prost_types::FieldMask;
use uuid::Uuid;
use warp_multi_agent_api as api;
use warp_multi_agent_api::client_action::{
    Action, AddMessagesToTask, AppendToMessageContent, CreateTask,
};
use warp_multi_agent_api::message::tool_call::apply_file_diffs::{FileDiff, NewFile};
use warp_multi_agent_api::message::tool_call::read_files::File as ReadFile;
use warp_multi_agent_api::message::tool_call::{
    ApplyFileDiffs, FileGlobV2, Grep, ReadFiles, RunShellCommand, Tool,
};
use warp_multi_agent_api::message::{AgentOutput, Message as MessageKind, ToolCall, UserQuery};
use warp_multi_agent_api::response_event::stream_finished::InternalError;
use warp_multi_agent_api::response_event::{
    ClientActions, StreamFinished, StreamInit, stream_finished,
};
use warp_multi_agent_api::{ClientAction, Message, ResponseEvent, Task};

use super::ResponseStream;
use crate::ai::agent::api::local_engine::llm::{
    ChatMessage, LlmModel, StreamingRequest, ToolCallRequest,
};

mod llm;

const OPENAI_API_BASE_URL: &str = "https://api.openai.com/v1";
const ANTHROPIC_API_BASE_URL: &str = "https://api.anthropic.com/v1";
const AGENT_OUTPUT_TEXT_PATH: &str = "agent_output.text";
const DEFAULT_MAX_OUTPUT_TOKENS: u32 = 8192;
const NO_ACCOUNT_MODEL_HINT: &str = "this model runs on Warp's servers, which requires a \
     Warp account. Configure a custom model endpoint or your own API key to use the agent \
     without an account";

/// Runs the agent loop locally, producing the same event stream the MAA server
/// would. Always returns a stream: setup and request failures surface as
/// `StreamFinished` internal errors so the conversation shows one clean,
/// terminal message instead of triggering transport retries.
pub(super) fn generate_local_multi_agent_output(request: &api::Request) -> ResponseStream {
    let state = match prepare(request) {
        Ok(engine) => LocalEngineState::Running(engine),
        Err(error) => LocalEngineState::Failed(format!("{error:#}")),
    };
    Box::pin(futures::stream::unfold(state, |state| async move {
        match state {
            LocalEngineState::Failed(message) => {
                Some((Ok(internal_error_event(message)), LocalEngineState::Done))
            }
            LocalEngineState::Running(mut engine) => engine
                .next_event()
                .await
                .map(|event| (Ok(event), LocalEngineState::Running(engine))),
            LocalEngineState::Done => None,
        }
    }))
}

enum LocalEngineState {
    Failed(String),
    Running(LocalEngine),
    Done,
}

/// Resolves the model the local engine should call: a user-configured custom
/// endpoint, or a first-party BYO API key.
fn resolve_model(request: &api::Request) -> Result<LlmModel, String> {
    let settings = request.settings.as_ref();
    let model_config = settings
        .and_then(|settings| settings.model_config.as_ref())
        .ok_or("no model selected")?;
    let selected = model_config.base.trim();
    if selected.is_empty() {
        return Err("no model selected".to_owned());
    }

    let providers = settings
        .and_then(|settings| settings.custom_model_providers.as_ref())
        .map(|providers| providers.providers.as_slice())
        .unwrap_or_default();
    let find_custom = |model_id: &str| {
        providers.iter().find_map(|provider| {
            let model = provider
                .models
                .iter()
                .find(|model| model.config_key == model_id)?;
            let schema = provider.schema.try_into().unwrap_or(
                api::request::settings::custom_model_providers::CustomEndpointSchema::OpenaiChatCompletions,
            );
            Some(LlmModel {
                base_url: provider.base_url.clone(),
                api_key: provider.api_key.clone(),
                schema,
                model_id: model.slug.clone(),
                reasoning_effort: (!model.reasoning_effort.is_empty())
                    .then(|| model.reasoning_effort.clone()),
            })
        })
    };
    if let Some(model) = find_custom(selected) {
        return Ok(model);
    }

    // Custom model routers reference another model by config key; resolve
    // through their default target.
    let routers = settings
        .and_then(|settings| settings.custom_model_routers.as_ref())
        .map(|routers| routers.routers.as_slice())
        .unwrap_or_default();
    if let Some(router) = routers.iter().find(|router| router.config_key == selected) {
        let target = match router.router.as_ref() {
            Some(
                api::request::settings::custom_model_routers::custom_model_router::Router::Complexity(complexity),
            ) => complexity.default.clone(),
            Some(
                api::request::settings::custom_model_routers::custom_model_router::Router::Prompt(prompt),
            ) => prompt.default.clone(),
            None => String::new(),
        };
        if !target.is_empty() && target != selected {
            if let Some(model) = find_custom(&target) {
                return Ok(model);
            }
            // The router may target a first-party model; fall through with the
            // resolved id so a BYO key can serve it below.
            return resolve_byo_key(settings, &target);
        }
    }

    resolve_byo_key(settings, selected)
}

fn resolve_byo_key(
    settings: Option<&api::request::Settings>,
    model_id: &str,
) -> Result<LlmModel, String> {
    let api_keys = settings
        .and_then(|settings| settings.api_keys.as_ref())
        .ok_or_else(|| NO_ACCOUNT_MODEL_HINT.to_owned())?;
    if !api_keys.anthropic.is_empty() {
        return Ok(LlmModel {
            base_url: ANTHROPIC_API_BASE_URL.to_owned(),
            api_key: api_keys.anthropic.clone(),
            schema:
                api::request::settings::custom_model_providers::CustomEndpointSchema::AnthropicMessages,
            model_id: model_id.to_owned(),
            reasoning_effort: None,
        });
    }
    if !api_keys.openai.is_empty() {
        return Ok(LlmModel {
            base_url: OPENAI_API_BASE_URL.to_owned(),
            api_key: api_keys.openai.clone(),
            schema:
                api::request::settings::custom_model_providers::CustomEndpointSchema::OpenaiChatCompletions,
            model_id: model_id.to_owned(),
            reasoning_effort: None,
        });
    }
    Err(NO_ACCOUNT_MODEL_HINT.to_owned())
}

/// Everything the streaming stage needs, resolved up front so failures surface
/// before any events are emitted.
struct LocalEngine {
    conversation_id: String,
    request_id: String,
    run_id: String,
    task_id: String,
    task_existed: bool,
    user_queries: Vec<String>,
    stream: BoxStream<'static, Result<llm::StreamChunk, String>>,
    output_message_open: bool,
    init_emitted: bool,
    opening_actions_emitted: bool,
    finished: bool,
}

fn prepare(request: &api::Request) -> anyhow::Result<LocalEngine> {
    let model = resolve_model(request).map_err(anyhow::Error::msg)?;

    let input = request
        .input
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("request has no input"))?;
    let user_queries = collect_user_queries(input);
    if user_queries.is_empty() && !has_tool_call_results(input) {
        return Err(anyhow::anyhow!(
            "the local agent only supports prompts and tool results; this request type \
             requires a Warp account"
        ));
    }

    let tasks = request
        .task_context
        .as_ref()
        .map(|context| context.tasks.as_slice())
        .unwrap_or_default();
    let root_task_id = tasks
        .iter()
        .find(|task| is_root_task(task))
        .map(|task| task.id.clone());
    let (task_id, task_existed) = match root_task_id {
        Some(id) => (id, true),
        None => (Uuid::new_v4().to_string(), false),
    };

    let conversation_id = request
        .metadata
        .as_ref()
        .filter(|metadata| !metadata.conversation_id.is_empty())
        .map(|metadata| metadata.conversation_id.clone())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let messages = build_chat_history(tasks, input);
    let system_prompt = build_system_prompt(input);
    let max_output_tokens = request
        .settings
        .as_ref()
        .and_then(|settings| settings.model_config.as_ref())
        .map(|config| config.base_model_context_window_limit)
        .filter(|limit| *limit > 0)
        .unwrap_or(DEFAULT_MAX_OUTPUT_TOKENS);

    let stream = llm::stream_completion(StreamingRequest {
        model,
        system_prompt,
        messages,
        max_output_tokens,
    });

    Ok(LocalEngine {
        conversation_id,
        request_id: Uuid::new_v4().to_string(),
        run_id: Uuid::new_v4().to_string(),
        task_id,
        task_existed,
        user_queries,
        stream,
        output_message_open: false,
        init_emitted: false,
        opening_actions_emitted: false,
        finished: false,
    })
}

impl LocalEngine {
    async fn next_event(&mut self) -> Option<ResponseEvent> {
        if self.finished {
            return None;
        }
        if !self.init_emitted {
            self.init_emitted = true;
            return Some(ResponseEvent {
                r#type: Some(api::response_event::Type::Init(StreamInit {
                    conversation_id: self.conversation_id.clone(),
                    request_id: self.request_id.clone(),
                    run_id: self.run_id.clone(),
                })),
            });
        }
        if !self.opening_actions_emitted {
            self.opening_actions_emitted = true;
            return Some(self.opening_event());
        }
        match self.stream.next().await {
            Some(Ok(chunk)) => Some(self.chunk_event(chunk)),
            Some(Err(message)) => {
                self.finished = true;
                Some(internal_error_event(message))
            }
            None => {
                self.finished = true;
                Some(done_event())
            }
        }
    }

    /// The server's opening salvo, mirrored: create the root task when it is
    /// still client-only and echo the user's queries as messages.
    fn opening_event(&self) -> ResponseEvent {
        let mut actions = Vec::new();
        if !self.task_existed {
            actions.push(create_task_action(self.task_id.clone()));
        }
        for query in &self.user_queries {
            actions.push(add_messages_action(
                self.task_id.clone(),
                vec![user_query_message(&self.task_id, &self.request_id, query)],
            ));
        }
        client_actions_event(actions)
    }

    fn chunk_event(&mut self, chunk: llm::StreamChunk) -> ResponseEvent {
        match chunk {
            llm::StreamChunk::Text(delta) => {
                if delta.is_empty() {
                    return client_actions_event(Vec::new());
                }
                if self.output_message_open {
                    client_actions_event(vec![append_text_action(
                        &self.task_id,
                        &self.request_id,
                        &self.output_message_id(),
                        &delta,
                    )])
                } else {
                    self.output_message_open = true;
                    client_actions_event(vec![add_messages_action(
                        self.task_id.clone(),
                        vec![output_message(&self.task_id, &self.request_id, &delta)],
                    )])
                }
            }
            llm::StreamChunk::ToolCall(tool) => {
                // Tool calls end the assistant's turn, mirroring MAA: the
                // client executes them and sends the results in the next
                // request.
                self.output_message_open = false;
                client_actions_event(vec![add_messages_action(
                    self.task_id.clone(),
                    vec![tool_call_message(&self.task_id, &self.request_id, tool)],
                )])
            }
        }
    }

    fn output_message_id(&self) -> String {
        format!("output-{}", self.request_id)
    }
}

fn is_root_task(task: &Task) -> bool {
    task.dependencies
        .as_ref()
        .is_none_or(|dependencies| dependencies.parent_task_id.is_empty())
}

fn collect_user_queries(input: &api::request::Input) -> Vec<String> {
    let Some(api::request::input::Type::UserInputs(user_inputs)) = input.r#type.as_ref() else {
        return Vec::new();
    };
    user_inputs
        .inputs
        .iter()
        .filter_map(|user_input| match user_input.input.as_ref() {
            Some(api::request::input::user_inputs::user_input::Input::UserQuery(query)) => {
                Some(query.query.clone())
            }
            _ => None,
        })
        .collect()
}

fn has_tool_call_results(input: &api::request::Input) -> bool {
    let Some(api::request::input::Type::UserInputs(user_inputs)) = input.r#type.as_ref() else {
        return false;
    };
    user_inputs.inputs.iter().any(|user_input| {
        matches!(
            user_input.input.as_ref(),
            Some(api::request::input::user_inputs::user_input::Input::ToolCallResult(_))
        )
    })
}

/// Flattens the task history plus this request's input into chat messages for
/// the model. Only the root task's linear message list is used; sub-agent
/// tasks and non-conversational messages are skipped.
fn build_chat_history(tasks: &[Task], input: &api::request::Input) -> Vec<ChatMessage> {
    let mut messages = Vec::new();
    let root = tasks.iter().find(|task| is_root_task(task));

    let mut pending_tool_calls: Vec<ToolCallRequest> = Vec::new();
    if let Some(root) = root {
        for message in &root.messages {
            match message.message.as_ref() {
                Some(MessageKind::UserQuery(query)) => {
                    flush_tool_calls(&mut messages, &mut pending_tool_calls);
                    if !query.query.is_empty() {
                        messages.push(ChatMessage::user(query.query.clone()));
                    }
                }
                Some(MessageKind::AgentOutput(output)) => {
                    flush_tool_calls(&mut messages, &mut pending_tool_calls);
                    if !output.text.is_empty() {
                        messages.push(ChatMessage::assistant(output.text.clone()));
                    }
                }
                Some(MessageKind::ToolCall(tool_call)) => {
                    if let Some(tool) = ToolCallRequest::from_proto(tool_call) {
                        pending_tool_calls.push(tool);
                    }
                }
                Some(MessageKind::ToolCallResult(result)) => {
                    flush_tool_calls(&mut messages, &mut pending_tool_calls);
                    messages.push(ChatMessage::tool_result(
                        result.tool_call_id.clone(),
                        llm::tool_result_text(result),
                    ));
                }
                _ => {}
            }
        }
    }
    flush_tool_calls(&mut messages, &mut pending_tool_calls);
    messages.extend(new_input_messages(input));
    messages
}

fn flush_tool_calls(
    messages: &mut Vec<ChatMessage>,
    pending_tool_calls: &mut Vec<ToolCallRequest>,
) {
    if pending_tool_calls.is_empty() {
        return;
    }
    let tool_calls = std::mem::take(pending_tool_calls);
    messages.push(ChatMessage::assistant_tool_calls(tool_calls));
}

fn new_input_messages(input: &api::request::Input) -> Vec<ChatMessage> {
    let Some(api::request::input::Type::UserInputs(user_inputs)) = input.r#type.as_ref() else {
        return Vec::new();
    };
    user_inputs
        .inputs
        .iter()
        .filter_map(|user_input| match user_input.input.as_ref() {
            Some(api::request::input::user_inputs::user_input::Input::UserQuery(query)) => {
                Some(ChatMessage::user(query.query.clone()))
            }
            Some(api::request::input::user_inputs::user_input::Input::ToolCallResult(result)) => {
                Some(ChatMessage::tool_result(
                    result.tool_call_id.clone(),
                    llm::input_tool_result_text(result),
                ))
            }
            _ => None,
        })
        .collect()
}

fn build_system_prompt(input: &api::request::Input) -> String {
    let mut prompt = String::from(
        "You are Warp, an AI-powered terminal agent. You help the user with software \
         engineering tasks: answering questions, writing and debugging code, and running \
         commands in their terminal.\n\n\
         When a task benefits from running commands, reading files, searching the \
         codebase, or editing files, use the provided tools instead of guessing. Prefer \
         read-only tools (reading files, searching) before making changes. When you run a \
         command, the user reviews it first; report what happened based on the tool \
         results. Keep answers concise and format code with markdown code blocks.\n",
    );

    if let Some(context) = input.context.as_ref() {
        let mut details = Vec::new();
        if let Some(directory) = context.directory.as_ref() {
            if !directory.pwd.is_empty() {
                details.push(format!("Working directory: {}", directory.pwd));
            }
            if !directory.home.is_empty() {
                details.push(format!("Home directory: {}", directory.home));
            }
        }
        if let Some(shell) = context.shell.as_ref()
            && !shell.name.is_empty()
        {
            let mut line = format!("Shell: {}", shell.name);
            if !shell.version.is_empty() {
                line.push_str(&format!(" {}", shell.version));
            }
            details.push(line);
        }
        if let Some(os) = context.operating_system.as_ref()
            && !os.platform.is_empty()
        {
            let mut line = format!("OS: {}", os.platform);
            if !os.distribution.is_empty() {
                line.push_str(&format!(" ({})", os.distribution));
            }
            details.push(line);
        }
        if let Some(time) = context.current_time.as_ref()
            && let Some(time) = chrono::DateTime::from_timestamp(
                time.seconds,
                time.nanos.try_into().unwrap_or_default(),
            )
        {
            details.push(format!("Current time: {}", time.to_rfc3339()));
        }
        if !details.is_empty() {
            prompt.push_str("\nEnvironment:\n");
            for detail in details {
                prompt.push_str(&format!("- {detail}\n"));
            }
        }
    }
    prompt
}

fn internal_error_event(message: String) -> ResponseEvent {
    ResponseEvent {
        r#type: Some(api::response_event::Type::Finished(StreamFinished {
            reason: Some(stream_finished::Reason::InternalError(InternalError {
                message,
            })),
            ..Default::default()
        })),
    }
}

fn done_event() -> ResponseEvent {
    ResponseEvent {
        r#type: Some(api::response_event::Type::Finished(StreamFinished {
            reason: Some(stream_finished::Reason::Done(stream_finished::Done {})),
            ..Default::default()
        })),
    }
}

fn client_actions_event(actions: Vec<ClientAction>) -> ResponseEvent {
    ResponseEvent {
        r#type: Some(api::response_event::Type::ClientActions(ClientActions {
            actions,
        })),
    }
}

fn create_task_action(task_id: String) -> ClientAction {
    ClientAction {
        action: Some(Action::CreateTask(CreateTask {
            task: Some(Task {
                id: task_id,
                ..Default::default()
            }),
        })),
    }
}

fn add_messages_action(task_id: String, messages: Vec<Message>) -> ClientAction {
    ClientAction {
        action: Some(Action::AddMessagesToTask(AddMessagesToTask {
            task_id,
            messages,
        })),
    }
}

fn append_text_action(
    task_id: &str,
    request_id: &str,
    message_id: &str,
    delta: &str,
) -> ClientAction {
    ClientAction {
        action: Some(Action::AppendToMessageContent(AppendToMessageContent {
            task_id: task_id.to_owned(),
            message: Some(Message {
                id: message_id.to_owned(),
                task_id: task_id.to_owned(),
                request_id: request_id.to_owned(),
                message: Some(MessageKind::AgentOutput(AgentOutput {
                    text: delta.to_owned(),
                })),
                ..Default::default()
            }),
            mask: Some(FieldMask {
                paths: vec![AGENT_OUTPUT_TEXT_PATH.to_owned()],
            }),
        })),
    }
}

fn user_query_message(task_id: &str, request_id: &str, query: &str) -> Message {
    new_message(
        task_id,
        request_id,
        MessageKind::UserQuery(UserQuery {
            query: query.to_owned(),
            ..Default::default()
        }),
    )
}

fn output_message(task_id: &str, request_id: &str, text: &str) -> Message {
    Message {
        id: format!("output-{request_id}"),
        message: Some(MessageKind::AgentOutput(AgentOutput {
            text: text.to_owned(),
        })),
        ..new_message(
            task_id,
            request_id,
            MessageKind::AgentOutput(AgentOutput::default()),
        )
    }
}

fn tool_call_message(task_id: &str, request_id: &str, tool: ToolCallRequest) -> Message {
    let tool_call = ToolCall {
        tool_call_id: tool.id.clone(),
        tool: Some(proto_tool(&tool)),
    };
    new_message(task_id, request_id, MessageKind::ToolCall(tool_call))
}

fn proto_tool(tool: &ToolCallRequest) -> Tool {
    let string_arg = |name: &str| {
        tool.arguments
            .get(name)
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    let string_array_arg = |name: &str| -> Vec<String> {
        tool.arguments
            .get(name)
            .and_then(serde_json::Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default()
    };
    match tool.name.as_str() {
        "read_files" => Tool::ReadFiles(ReadFiles {
            files: string_array_arg("files")
                .into_iter()
                .map(|name| ReadFile {
                    name,
                    line_ranges: Vec::new(),
                })
                .collect(),
        }),
        "grep" => Tool::Grep(Grep {
            queries: string_array_arg("queries"),
            path: string_arg("path"),
        }),
        "file_glob" => Tool::FileGlobV2(FileGlobV2 {
            patterns: string_array_arg("patterns"),
            search_dir: string_arg("search_dir"),
            ..Default::default()
        }),
        "apply_file_diffs" => Tool::ApplyFileDiffs(ApplyFileDiffs {
            summary: string_arg("summary"),
            diffs: json_array_arg(&tool.arguments, "diffs", |item| {
                Some(FileDiff {
                    file_path: item.get("file_path")?.as_str()?.to_owned(),
                    search: item
                        .get("search")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_owned(),
                    replace: item
                        .get("replace")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_owned(),
                })
            }),
            new_files: json_array_arg(&tool.arguments, "new_files", |item| {
                Some(NewFile {
                    file_path: item.get("file_path")?.as_str()?.to_owned(),
                    content: item
                        .get("content")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_owned(),
                    ..Default::default()
                })
            }),
            ..Default::default()
        }),
        // "run_shell_command" and anything unrecognized: a shell command is
        // the most useful degradation for an unknown tool name.
        _ => Tool::RunShellCommand(RunShellCommand {
            command: string_arg("command"),
            ..Default::default()
        }),
    }
}

fn json_array_arg<T>(
    arguments: &serde_json::Map<String, serde_json::Value>,
    name: &str,
    map_item: impl Fn(&serde_json::Value) -> Option<T>,
) -> Vec<T> {
    arguments
        .get(name)
        .and_then(serde_json::Value::as_array)
        .map(|items| items.iter().filter_map(map_item).collect())
        .unwrap_or_default()
}

fn new_message(task_id: &str, request_id: &str, kind: MessageKind) -> Message {
    Message {
        id: Uuid::new_v4().to_string(),
        task_id: task_id.to_owned(),
        request_id: request_id.to_owned(),
        timestamp: Some(prost_types::Timestamp::from(std::time::SystemTime::now())),
        message: Some(kind),
        ..Default::default()
    }
}

#[cfg(test)]
#[path = "local_engine_tests.rs"]
mod tests;
