use futures::StreamExt as _;
use warp_multi_agent_api as api;

use super::*;

fn custom_provider_request(model_config_key: &str) -> api::Request {
    api::Request {
        task_context: None,
        input: Some(api::request::Input {
            context: None,
            r#type: Some(api::request::input::Type::UserInputs(
                api::request::input::UserInputs {
                    inputs: vec![api::request::input::user_inputs::UserInput {
                        input: Some(
                            api::request::input::user_inputs::user_input::Input::UserQuery(
                                api::request::input::UserQuery {
                                    query: "hello".to_owned(),
                                    ..Default::default()
                                },
                            ),
                        ),
                    }],
                },
            )),
        }),
        settings: Some(api::request::Settings {
            model_config: Some(api::request::settings::ModelConfig {
                base: model_config_key.to_owned(),
                ..Default::default()
            }),
            custom_model_providers: Some(api::request::settings::CustomModelProviders {
                providers: vec![api::request::settings::custom_model_providers::CustomModelProvider {
                    base_url: "http://localhost:11434/v1".to_owned(),
                    api_key: String::new(),
                    models: vec![api::request::settings::custom_model_providers::CustomModel {
                        slug: "llama3".to_owned(),
                        config_key: "custom-llama".to_owned(),
                        reasoning_effort: String::new(),
                    }],
                    schema: api::request::settings::custom_model_providers::CustomEndpointSchema::OpenaiChatCompletions as i32,
                }],
            }),
            ..Default::default()
        }),
        metadata: None,
        existing_suggestions: None,
        mcp_context: None,
    }
}

#[test]
fn resolves_custom_model_by_config_key() {
    let model = resolve_model(&custom_provider_request("custom-llama")).unwrap();
    assert_eq!(model.base_url, "http://localhost:11434/v1");
    assert_eq!(model.model_id, "llama3");
}

#[test]
fn resolves_router_default_to_custom_model() {
    let mut request = custom_provider_request("router-key");
    request.settings.as_mut().unwrap().custom_model_routers =
        Some(api::request::settings::CustomModelRouters {
            routers: vec![api::request::settings::custom_model_routers::CustomModelRouter {
                config_key: "router-key".to_owned(),
                name: "My router".to_owned(),
                router: Some(
                    api::request::settings::custom_model_routers::custom_model_router::Router::Complexity(
                        api::request::settings::custom_model_routers::ComplexityBasedRouter {
                            default: "custom-llama".to_owned(),
                            easy: String::new(),
                            medium: String::new(),
                            hard: String::new(),
                        },
                    ),
                ),
            }],
        });
    let model = resolve_model(&request).unwrap();
    assert_eq!(model.model_id, "llama3");
}

#[test]
fn falls_back_to_byo_anthropic_key() {
    let mut request = custom_provider_request("claude-4-5-sonnet");
    request.settings.as_mut().unwrap().api_keys = Some(api::request::settings::ApiKeys {
        anthropic: "sk-ant-test".to_owned(),
        ..Default::default()
    });
    let model = resolve_model(&request).unwrap();
    assert_eq!(model.base_url, "https://api.anthropic.com/v1");
    assert_eq!(model.api_key, "sk-ant-test");
    assert_eq!(model.model_id, "claude-4-5-sonnet");
}

#[test]
fn no_local_model_yields_actionable_error_event() {
    let request = custom_provider_request("some-warp-hosted-model");
    let stream = generate_local_multi_agent_output(&request, None);
    let mut stream = futures::executor::block_on_stream(stream);
    let event = stream.next().unwrap().unwrap();
    drop(stream);
    let Some(api::response_event::Type::Finished(finished)) = event.r#type else {
        panic!("expected a finished event");
    };
    let Some(api::response_event::stream_finished::Reason::InternalError(error)) = finished.reason
    else {
        panic!("expected an internal error");
    };
    assert!(error.message.contains("custom model endpoint"));
}

#[test]
fn stream_opens_with_init_task_and_user_query_echo() {
    let request = custom_provider_request("custom-llama");
    let stream = generate_local_multi_agent_output(&request, None);
    let mut stream = futures::executor::block_on_stream(stream);
    let init = stream.next().unwrap().unwrap();
    assert!(matches!(
        init.r#type,
        Some(api::response_event::Type::Init(_))
    ));
    let opening = stream.next().unwrap().unwrap();
    drop(stream);

    let Some(api::response_event::Type::ClientActions(actions)) = opening.r#type else {
        panic!("expected client actions");
    };
    assert_eq!(actions.actions.len(), 2);
    assert!(matches!(
        actions.actions[0].action,
        Some(api::client_action::Action::CreateTask(_))
    ));
    let Some(api::client_action::Action::AddMessagesToTask(add_messages)) =
        &actions.actions[1].action
    else {
        panic!("expected an add-messages action");
    };
    assert!(matches!(
        add_messages.messages[0].message,
        Some(api::message::Message::UserQuery(_))
    ));
}

#[test]
fn chat_history_flattens_messages_and_new_input() {
    let task = api::Task {
        id: "root".to_owned(),
        messages: vec![
            message(api::message::Message::UserQuery(api::message::UserQuery {
                query: "list the files".to_owned(),
                ..Default::default()
            })),
            message(api::message::Message::ToolCall(api::message::ToolCall {
                tool_call_id: "call-1".to_owned(),
                tool: Some(api::message::tool_call::Tool::RunShellCommand(
                    api::message::tool_call::RunShellCommand {
                        command: "ls".to_owned(),
                        ..Default::default()
                    },
                )),
            })),
            message(api::message::Message::ToolCallResult(
                api::message::ToolCallResult {
                    tool_call_id: "call-1".to_owned(),
                    result: Some(api::message::tool_call_result::Result::RunShellCommand(
                        api::RunShellCommandResult {
                            result: Some(api::run_shell_command_result::Result::CommandFinished(
                                api::ShellCommandFinished {
                                    output: "README.md".to_owned(),
                                    exit_code: 0,
                                    ..Default::default()
                                },
                            )),
                            ..Default::default()
                        },
                    )),
                    ..Default::default()
                },
            )),
            message(api::message::Message::AgentOutput(
                api::message::AgentOutput {
                    text: "There is one file.".to_owned(),
                },
            )),
        ],
        ..Default::default()
    };
    let request = custom_provider_request("custom-llama");
    let input = request.input.as_ref().unwrap();
    let history = build_chat_history(&[task], input);
    assert_eq!(history.len(), 5);
    assert!(matches!(&history[0], ChatMessage::User { text, .. } if text == "list the files"));
    assert!(
        matches!(&history[1], ChatMessage::AssistantToolCalls(calls) if calls.len() == 1
        && calls[0].name == "run_shell_command")
    );
    assert!(
        matches!(&history[2], ChatMessage::ToolResult { id, content }
        if id == "call-1" && content.contains("README.md"))
    );
    assert!(matches!(&history[3], ChatMessage::Assistant(text) if text == "There is one file."));
    // The request's new user query is appended for the model to answer.
    assert!(matches!(&history[4], ChatMessage::User { text, .. } if text == "hello"));
}

fn message(kind: api::message::Message) -> api::Message {
    api::Message {
        id: uuid::Uuid::new_v4().to_string(),
        task_id: "root".to_owned(),
        request_id: "request-1".to_owned(),
        message: Some(kind),
        ..Default::default()
    }
}

#[test]
fn streams_completion_from_an_openai_compatible_endpoint() {
    use std::io::{Read as _, Write as _};
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = vec![0u8; 4096];
        let read = stream.read(&mut request).unwrap();
        let request = String::from_utf8_lossy(&request[..read]).to_string();
        assert!(request.contains("POST /v1/chat/completions"), "{request}");
        assert!(request.contains("\"stream\":true"), "{request}");
        assert!(request.contains("\"tools\""), "{request}");

        let body = concat!(
            "data: {\"choices\":[{\"delta\":{\"content\":\"Hi \"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"there\"}}]}\n\n",
            "data: [DONE]\n\n",
        );
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(response.as_bytes()).unwrap();
    });

    let mut request = custom_provider_request_with_url(&format!("http://127.0.0.1:{port}/v1"));
    // The mock speaks plain HTTP; ensure the request carries a user query.
    request.settings.as_mut().unwrap().custom_model_providers =
        Some(api::request::settings::CustomModelProviders {
            providers: vec![api::request::settings::custom_model_providers::CustomModelProvider {
                base_url: format!("http://127.0.0.1:{port}/v1"),
                api_key: String::new(),
                models: vec![api::request::settings::custom_model_providers::CustomModel {
                    slug: "mock-model".to_owned(),
                    config_key: "custom-mock".to_owned(),
                    reasoning_effort: String::new(),
                }],
                schema: api::request::settings::custom_model_providers::CustomEndpointSchema::OpenaiChatCompletions as i32,
            }],
        });
    request.settings.as_mut().unwrap().model_config = Some(api::request::settings::ModelConfig {
        base: "custom-mock".to_owned(),
        ..Default::default()
    });

    let events: Vec<_> = futures::executor::block_on(async {
        generate_local_multi_agent_output(&request, None)
            .then(|event| async move { event })
            .collect()
            .await
    });
    server.join().unwrap();

    assert!(!events.is_empty());
    assert!(matches!(
        events[0].as_ref().unwrap().r#type,
        Some(api::response_event::Type::Init(_))
    ));
    let text: String = events
        .iter()
        .filter_map(|event| event.as_ref().ok())
        .filter_map(|event| {
            let Some(api::response_event::Type::ClientActions(actions)) = &event.r#type else {
                return None;
            };
            Some(actions.actions.iter().filter_map(|action| {
                match &action.action {
                    Some(api::client_action::Action::AddMessagesToTask(add)) => add
                        .messages
                        .iter()
                        .find_map(|message| match &message.message {
                            Some(api::message::Message::AgentOutput(output)) => {
                                Some(output.text.clone())
                            }
                            _ => None,
                        }),
                    Some(api::client_action::Action::AppendToMessageContent(append)) => {
                        match &append.message.as_ref().and_then(|m| m.message.as_ref()) {
                            Some(api::message::Message::AgentOutput(output)) => {
                                Some(output.text.clone())
                            }
                            _ => None,
                        }
                    }
                    _ => None,
                }
            }))
        })
        .flatten()
        .collect();
    assert_eq!(text, "Hi there");
    assert!(matches!(
        events
            .last()
            .and_then(|event| event.as_ref().ok())
            .and_then(|event| event.r#type.as_ref()),
        Some(api::response_event::Type::Finished(_))
    ));
}

fn custom_provider_request_with_url(_base_url: &str) -> api::Request {
    custom_provider_request("custom-mock")
}

#[test]
fn streams_completion_from_an_anthropic_endpoint() {
    use std::io::{Read as _, Write as _};
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = vec![0u8; 8192];
        let read = stream.read(&mut request).unwrap();
        let request = String::from_utf8_lossy(&request[..read]).to_string();
        // The base URL has no /v1 suffix, so the heuristic must add it.
        assert!(
            request.contains("POST /v1/messages"),
            "unexpected path in request: {request}"
        );

        let body = concat!(
            "data: {\"type\":\"message_start\"}\n\n",
            "data: {\"type\":\"content_block_start\",\"content_block\":{\"type\":\"text\"}}\n\n",
            "data: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"Bonjour\"}}\n\n",
            "data: {\"type\":\"content_block_stop\"}\n\n",
            "data: {\"type\":\"message_stop\"}\n\n",
        );
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(response.as_bytes()).unwrap();
    });

    let request = anthropic_request(&format!("http://127.0.0.1:{port}"));
    let events: Vec<_> = futures::executor::block_on(async {
        generate_local_multi_agent_output(&request, None)
            .then(|event| async move { event })
            .collect()
            .await
    });
    server.join().unwrap();

    let text: String = events
        .iter()
        .filter_map(|event| event.as_ref().ok())
        .filter_map(|event| {
            let Some(api::response_event::Type::ClientActions(actions)) = &event.r#type else {
                return None;
            };
            Some(actions.actions.iter().filter_map(|action| {
                let Some(api::client_action::Action::AddMessagesToTask(add)) = &action.action
                else {
                    return None;
                };
                add.messages
                    .iter()
                    .find_map(|message| match &message.message {
                        Some(api::message::Message::AgentOutput(output)) => {
                            Some(output.text.clone())
                        }
                        _ => None,
                    })
            }))
        })
        .flatten()
        .collect();
    assert_eq!(text, "Bonjour");
}

fn anthropic_request(base_url: &str) -> api::Request {
    let mut request = custom_provider_request("custom-mock");
    request.settings.as_mut().unwrap().custom_model_providers =
        Some(api::request::settings::CustomModelProviders {
            providers: vec![api::request::settings::custom_model_providers::CustomModelProvider {
                base_url: base_url.to_owned(),
                api_key: "test-key".to_owned(),
                models: vec![api::request::settings::custom_model_providers::CustomModel {
                    slug: "claude-mock".to_owned(),
                    config_key: "custom-mock".to_owned(),
                    reasoning_effort: String::new(),
                }],
                schema:
                    api::request::settings::custom_model_providers::CustomEndpointSchema::AnthropicMessages
                        as i32,
            }],
        });
    request
}

#[test]
fn custom_model_settings_reach_the_streaming_request() {
    use std::io::{Read as _, Write as _};
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = vec![0u8; 16384];
        let read = stream.read(&mut request).unwrap();
        let request = String::from_utf8_lossy(&request[..read]).to_string();
        let body = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\nContent-Length: 0\r\n\r\n";
        stream.write_all(body.as_bytes()).unwrap();
        request
    });

    let request = anthropic_request(&format!("http://127.0.0.1:{port}"));
    let custom_model = ai::api_keys::CustomEndpointModel {
        name: "claude-mock".to_owned(),
        alias: None,
        config_key: "custom-mock".to_owned(),
        context_window: Some(200_000),
        max_output_tokens: Some(4096),
        reasoning_effort: Some("high".to_owned()),
        supports_images: true,
    };
    let events: Vec<_> = futures::executor::block_on(async {
        generate_local_multi_agent_output(&request, Some(&custom_model))
            .then(|event| async move { event })
            .collect()
            .await
    });
    let received_request = server.join().unwrap();

    // The configured settings reach the wire: the output cap and the
    // thinking budget derived from "high", clamped below max_tokens.
    assert!(
        received_request.contains("POST /v1/messages"),
        "{received_request}"
    );
    assert!(
        received_request.contains("\"max_tokens\":4096"),
        "{received_request}"
    );
    assert!(
        received_request.contains("\"budget_tokens\":3072"),
        "{received_request}"
    );
    assert!(!events.is_empty());
}

#[test]
fn truncate_history_drops_oldest_and_avoids_orphan_tool_results() {
    let mut messages = vec![
        ChatMessage::user("first question"),
        ChatMessage::assistant_tool_calls(vec![ToolCallRequest {
            id: "call-1".to_owned(),
            name: "run_shell_command".to_owned(),
            arguments: serde_json::json!({"command": "echo a really long output that takes many tokens to represent"})
                .as_object()
                .cloned()
                .unwrap(),
        }]),
        ChatMessage::tool_result("call-1", "a really long output ".repeat(50)),
        ChatMessage::user("second question"),
    ];
    truncate_history(&mut messages, 64);
    // The oldest exchange is dropped; the remaining suffix starts with a
    // user message, not an orphan tool result.
    assert!(matches!(&messages[0], ChatMessage::User { text, .. } if text == "second question"));
    assert_eq!(messages.len(), 1);
}

#[test]
fn truncate_history_keeps_everything_when_it_fits() {
    let mut messages = vec![ChatMessage::user("hello"), ChatMessage::assistant("hi")];
    truncate_history(&mut messages, 100_000);
    assert_eq!(messages.len(), 2);
}

#[test]
fn attach_input_images_targets_the_last_user_message() {
    let image = || api::input_context::Image {
        data: b"aGVsbG8=".to_vec(),
        mime_type: "image/png".to_owned(),
    };
    let mut input = custom_provider_request("custom-llama").input.unwrap();
    input.context = Some(api::InputContext {
        images: vec![image(), image()],
        ..Default::default()
    });
    let mut messages = vec![
        ChatMessage::user("earlier"),
        ChatMessage::assistant("answer"),
        ChatMessage::user("what is in these?"),
    ];
    attach_input_images(&mut messages, &input);
    match &messages[2] {
        ChatMessage::User { images, .. } => assert_eq!(images.len(), 2),
        _ => panic!("expected a user message"),
    }
    assert!(matches!(&messages[0], ChatMessage::User { images, .. } if images.is_empty()));
}
