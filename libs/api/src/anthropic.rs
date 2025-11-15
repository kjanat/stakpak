use super::ApiStreamError;
use eventsource_stream::Eventsource;
use futures_util::Stream;
use futures_util::StreamExt;
use reqwest::header::HeaderMap;
use reqwest::{Client as ReqwestClient, Error as ReqwestError};
use serde::{Deserialize, Serialize};
use stakpak_shared::models::integrations::openai::{
    AgentModel, ChatCompletionChoice, ChatCompletionResponse, ChatCompletionStreamChoice,
    ChatCompletionStreamResponse, ChatMessage, ChatMessageDelta, FinishReason, FunctionCall,
    FunctionCallDelta, MessageContent, PromptTokensDetails, Role, Tool, ToolCall,
    ToolCallDelta, Usage,
};
use stakpak_shared::tls_client::TlsClientConfig;
use stakpak_shared::tls_client::create_tls_client;
use std::sync::Mutex;

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";

#[derive(Clone, Debug)]
pub enum AnthropicAuth {
    ApiKey(String),
    OAuth(AnthropicOAuthConfig),
}

#[derive(Clone, Debug)]
pub struct AnthropicOAuthConfig {
    pub refresh_token: String,
    pub access_token: String,
    pub expires: u64, // milliseconds since Unix epoch
}

#[derive(Clone, Debug)]
pub struct AnthropicClientConfig {
    pub auth: AnthropicAuth,
}

#[derive(Debug)]
pub struct AnthropicClient {
    client: ReqwestClient,
    auth: Mutex<AnthropicAuth>,
}

impl Clone for AnthropicClient {
    fn clone(&self) -> Self {
        let auth = self
            .auth
            .lock()
            .expect("Mutex poisoned during clone")
            .clone();
        Self {
            client: self.client.clone(),
            auth: Mutex::new(auth),
        }
    }
}

#[derive(Serialize, Debug)]
struct AnthropicRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<AnthropicTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct AnthropicMessage {
    role: String,
    content: AnthropicContent,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
enum AnthropicContent {
    Text(String),
    Blocks(Vec<AnthropicContentBlock>),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
enum AnthropicContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },
}

#[derive(Serialize, Debug, Clone)]
struct AnthropicTool {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    input_schema: serde_json::Value,
}

#[derive(Deserialize, Debug)]
struct AnthropicResponse {
    id: String,
    #[serde(rename = "type")]
    #[allow(dead_code)]
    response_type: String,
    #[allow(dead_code)]
    role: String,
    content: Vec<AnthropicContentBlock>,
    model: String,
    stop_reason: Option<String>,
    usage: AnthropicUsage,
}

#[derive(Deserialize, Debug)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

impl AnthropicClient {
    pub fn new(config: &AnthropicClientConfig) -> Result<Self, String> {
        // Base headers that are always needed
        let mut headers = HeaderMap::new();
        headers.insert(
            "anthropic-version",
            ANTHROPIC_VERSION
                .parse()
                .map_err(|e| format!("Invalid version header: {e}"))?,
        );
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            "application/json"
                .parse()
                .map_err(|e| format!("Invalid content type: {e}"))?,
        );
        headers.insert(
            reqwest::header::USER_AGENT,
            format!("Stakpak/{}", env!("CARGO_PKG_VERSION"))
                .parse()
                .map_err(|e| format!("Invalid user agent: {e}"))?,
        );

        let client = create_tls_client(
            TlsClientConfig::default()
                .with_headers(headers)
                .with_timeout(std::time::Duration::from_secs(300)),
        )?;

        Ok(Self {
            client,
            auth: Mutex::new(config.auth.clone()),
        })
    }

    async fn get_valid_access_token(&self) -> Result<String, String> {
        // First, check if we need to refresh
        let should_refresh = {
            let auth = self
                .auth
                .lock()
                .map_err(|e| format!("Lock poisoned: {e}"))?;
            match &*auth {
                AnthropicAuth::ApiKey(_) => false,
                AnthropicAuth::OAuth(oauth) => {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_else(|_| std::time::Duration::from_secs(0))
                        .as_millis()
                        .min(u64::MAX as u128) as u64;

                    // Refresh if token expires within 5 minutes
                    oauth.expires < now + (5 * 60 * 1000)
                }
            }
        };

        if should_refresh {
            self.refresh_token().await?;
        }

        // Get the token
        let auth = self
            .auth
            .lock()
            .map_err(|e| format!("Lock poisoned: {e}"))?;
        match &*auth {
            AnthropicAuth::ApiKey(key) => Ok(key.clone()),
            AnthropicAuth::OAuth(oauth) => Ok(oauth.access_token.clone()),
        }
    }

    async fn refresh_token(&self) -> Result<(), String> {
        let refresh_token = {
            let auth = self
                .auth
                .lock()
                .map_err(|e| format!("Lock poisoned: {e}"))?;
            match &*auth {
                AnthropicAuth::OAuth(oauth) => oauth.refresh_token.clone(),
                AnthropicAuth::ApiKey(_) => return Err("Not using OAuth authentication".to_string()),
            }
        };

        let response = self
            .client
            .post("https://console.anthropic.com/v1/oauth/token")
            .json(&serde_json::json!({
                "grant_type": "refresh_token",
                "refresh_token": refresh_token,
                "client_id": "9d1c250a-e61b-44d9-88ed-5944d1962f5e"
            }))
            .send()
            .await
            .map_err(|e| format!("Token refresh failed: {e}"))?;

        if !response.status().is_success() {
            return Err(format!("Token refresh returned {}", response.status()));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse refresh response: {e}"))?;

        // Update the auth with new tokens
        let mut auth = self
            .auth
            .lock()
            .map_err(|e| format!("Lock poisoned: {e}"))?;
        if let AnthropicAuth::OAuth(oauth_mut) = &mut *auth {
            oauth_mut.access_token = json["access_token"]
                .as_str()
                .ok_or("Missing access_token")?
                .to_string();
            oauth_mut.refresh_token = json["refresh_token"]
                .as_str()
                .unwrap_or(&oauth_mut.refresh_token)
                .to_string();
            oauth_mut.expires = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_else(|_| std::time::Duration::from_secs(0))
                .as_millis()
                .min(u64::MAX as u128) as u64
                + (json["expires_in"].as_u64().unwrap_or(3600) * 1000);
        }

        Ok(())
    }

    fn map_model_to_anthropic(model: &AgentModel) -> String {
        match model {
            AgentModel::Smart => "claude-sonnet-4-20250514".to_string(),
            AgentModel::Eco => "claude-haiku-4-20250605".to_string(),
        }
    }

    fn convert_messages_to_anthropic(
        messages: Vec<ChatMessage>,
    ) -> Result<(Vec<AnthropicMessage>, Option<String>), String> {
        let mut anthropic_messages = Vec::new();
        let mut system_message = None;

        for msg in messages {
            match msg.role {
                Role::System => {
                    // Extract system message - Anthropic handles it separately
                    if let Some(content) = msg.content {
                        system_message = Some(content.to_string());
                    }
                }
                Role::User => {
                    let content = match msg.content {
                        Some(MessageContent::String(text)) => AnthropicContent::Text(text),
                        Some(MessageContent::Array(parts)) => {
                            let blocks: Vec<AnthropicContentBlock> = parts
                                .into_iter()
                                .filter_map(|part| {
                                    part.text.map(|text| AnthropicContentBlock::Text { text })
                                })
                                .collect();
                            AnthropicContent::Blocks(blocks)
                        }
                        None => AnthropicContent::Text(String::new()),
                    };
                    anthropic_messages.push(AnthropicMessage {
                        role: "user".to_string(),
                        content,
                    });
                }
                Role::Assistant => {
                    let mut blocks = Vec::new();

                    // Add text content if present
                    if let Some(content) = msg.content {
                        match content {
                            MessageContent::String(text) if !text.is_empty() => {
                                blocks.push(AnthropicContentBlock::Text { text });
                            }
                            MessageContent::Array(parts) => {
                                for part in parts {
                                    if let Some(text) = part.text
                                        && !text.is_empty()
                                    {
                                        blocks.push(AnthropicContentBlock::Text { text });
                                    }
                                }
                            }
                            MessageContent::String(_) => {}
                        }
                    }

                    // Add tool calls if present
                    if let Some(tool_calls) = msg.tool_calls {
                        for tool_call in tool_calls {
                            let input: serde_json::Value =
                                serde_json::from_str(&tool_call.function.arguments)
                                    .unwrap_or_else(|_| serde_json::json!({}));
                            blocks.push(AnthropicContentBlock::ToolUse {
                                id: tool_call.id,
                                name: tool_call.function.name,
                                input,
                            });
                        }
                    }

                    if !blocks.is_empty() {
                        anthropic_messages.push(AnthropicMessage {
                            role: "assistant".to_string(),
                            content: AnthropicContent::Blocks(blocks),
                        });
                    }
                }
                Role::Tool => {
                    // Tool result message
                    if let Some(tool_call_id) = msg.tool_call_id {
                        let content_text = msg.content.map(|c| c.to_string()).unwrap_or_default();

                        anthropic_messages.push(AnthropicMessage {
                            role: "user".to_string(),
                            content: AnthropicContent::Blocks(vec![
                                AnthropicContentBlock::ToolResult {
                                    tool_use_id: tool_call_id,
                                    content: content_text,
                                    is_error: None,
                                },
                            ]),
                        });
                    }
                }
                Role::Developer => {
                    // Skip other roles
                }
            }
        }

        Ok((anthropic_messages, system_message))
    }

    fn convert_tools_to_anthropic(tools: Option<Vec<Tool>>) -> Option<Vec<AnthropicTool>> {
        tools.map(|tools_vec| {
            tools_vec
                .into_iter()
                .map(|tool| AnthropicTool {
                    name: tool.function.name,
                    description: tool.function.description,
                    input_schema: tool.function.parameters,
                })
                .collect()
        })
    }

    fn convert_anthropic_response_to_openai(response: AnthropicResponse) -> ChatCompletionResponse {
        let mut text_content = Vec::new();
        let mut tool_calls = Vec::new();

        for block in response.content {
            match block {
                AnthropicContentBlock::Text { text } => {
                    text_content.push(text);
                }
                AnthropicContentBlock::ToolUse { id, name, input } => {
                    tool_calls.push(ToolCall {
                        id,
                        r#type: "function".to_string(),
                        function: FunctionCall {
                            name,
                            arguments: input.to_string(),
                        },
                    });
                }
                AnthropicContentBlock::ToolResult { .. } => {}
            }
        }

        let content = if !text_content.is_empty() {
            Some(MessageContent::String(text_content.join("\n")))
        } else {
            None
        };

        let tool_calls_opt = if !tool_calls.is_empty() {
            Some(tool_calls)
        } else {
            None
        };

        let finish_reason = match response.stop_reason.as_deref() {
            Some("end_turn") => FinishReason::Stop,
            Some("max_tokens") => FinishReason::Length,
            Some("tool_use") => FinishReason::ToolCalls,
            Some(_) | None => FinishReason::Stop,
        };

        ChatCompletionResponse {
            id: response.id,
            object: "chat.completion".to_string(),
            created: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            model: match response.model.as_str() {
                m if m.contains("sonnet") => AgentModel::Smart,
                m if m.contains("haiku") => AgentModel::Eco,
                _ => AgentModel::Smart,
            },
            choices: vec![ChatCompletionChoice {
                index: 0,
                message: ChatMessage {
                    role: Role::Assistant,
                    content,
                    name: None,
                    tool_calls: tool_calls_opt,
                    tool_call_id: None,
                },
                logprobs: None,
                finish_reason,
            }],
            usage: Usage {
                prompt_tokens: response.usage.input_tokens,
                completion_tokens: response.usage.output_tokens,
                total_tokens: response.usage.input_tokens + response.usage.output_tokens,
                prompt_tokens_details: Some(PromptTokensDetails {
                    input_tokens: Some(response.usage.input_tokens),
                    output_tokens: Some(response.usage.output_tokens),
                    cache_read_input_tokens: None,
                    cache_write_input_tokens: None,
                }),
            },
            system_fingerprint: None,
        }
    }

    pub async fn chat_completion(
        &self,
        model: AgentModel,
        messages: Vec<ChatMessage>,
        tools: Option<Vec<Tool>>,
    ) -> Result<ChatCompletionResponse, String> {
        let (anthropic_messages, system_message) = Self::convert_messages_to_anthropic(messages)?;
        let anthropic_tools = Self::convert_tools_to_anthropic(tools);

        let request = AnthropicRequest {
            model: Self::map_model_to_anthropic(&model),
            messages: anthropic_messages,
            max_tokens: Some(8192), // Default max tokens for Anthropic
            temperature: None,
            top_p: None,
            stream: Some(false),
            tools: anthropic_tools,
            system: system_message,
        };

        // Get valid token and set up headers based on auth type
        let token = self.get_valid_access_token().await?;
        let mut request_builder = self.client.post(ANTHROPIC_API_URL);

        let is_oauth = {
            let auth = self
                .auth
                .lock()
                .map_err(|e| format!("Lock poisoned: {e}"))?;
            matches!(&*auth, AnthropicAuth::OAuth(_))
        };

        if is_oauth {
            request_builder = request_builder
                .header("authorization", format!("Bearer {token}"))
                .header(
                    "anthropic-beta",
                    "oauth-2025-04-20,claude-code-20250219,interleaved-thinking-2025-05-14,fine-grained-tool-streaming-2025-05-14"
                );
        } else {
            request_builder = request_builder.header("x-api-key", token);
        }

        let response = request_builder
            .json(&request)
            .send()
            .await
            .map_err(|e: ReqwestError| e.to_string())?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!("Anthropic API error ({status}): {error_text}"));
        }

        let anthropic_response: AnthropicResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse Anthropic response: {e}"))?;

        Ok(Self::convert_anthropic_response_to_openai(
            anthropic_response,
        ))
    }

    pub async fn chat_completion_stream(
        &self,
        model: AgentModel,
        messages: Vec<ChatMessage>,
        tools: Option<Vec<Tool>>,
        _headers: Option<HeaderMap>,
    ) -> Result<
        (
            impl Stream<Item = Result<ChatCompletionStreamResponse, ApiStreamError>>,
            Option<String>,
        ),
        String,
    > {
        let (anthropic_messages, system_message) = Self::convert_messages_to_anthropic(messages)?;
        let anthropic_tools = Self::convert_tools_to_anthropic(tools);

        let request = AnthropicRequest {
            model: Self::map_model_to_anthropic(&model),
            messages: anthropic_messages,
            max_tokens: Some(8192),
            temperature: None,
            top_p: None,
            stream: Some(true),
            tools: anthropic_tools,
            system: system_message,
        };

        // Get valid token and set up headers based on auth type
        let token = self.get_valid_access_token().await?;
        let mut request_builder = self.client.post(ANTHROPIC_API_URL);

        let is_oauth = {
            let auth = self
                .auth
                .lock()
                .map_err(|e| format!("Lock poisoned: {e}"))?;
            matches!(&*auth, AnthropicAuth::OAuth(_))
        };

        if is_oauth {
            request_builder = request_builder
                .header("authorization", format!("Bearer {token}"))
                .header(
                    "anthropic-beta",
                    "oauth-2025-04-20,claude-code-20250219,interleaved-thinking-2025-05-14,fine-grained-tool-streaming-2025-05-14"
                );
        } else {
            request_builder = request_builder.header("x-api-key", token);
        }

        let response = request_builder
            .json(&request)
            .send()
            .await
            .map_err(|e: ReqwestError| e.to_string())?;

        let request_id = response
            .headers()
            .get("request-id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!("Anthropic API error ({status}): {error_text}"));
        }

        let stream = response.bytes_stream().eventsource().map(|event| {
            event
                .map_err(|err| {
                    eprintln!("stream: failed to read response: {err:?}");
                    ApiStreamError::Unknown("Failed to read response".to_string())
                })
                .and_then(|event| {
                    Self::convert_anthropic_stream_event_to_openai(&event.data, &event.event)
                })
        });

        Ok((stream, request_id))
    }

    fn convert_anthropic_stream_event_to_openai(
        data: &str,
        event_type: &str,
    ) -> Result<ChatCompletionStreamResponse, ApiStreamError> {
        // Parse the event data to extract model if available
        let event: serde_json::Value = serde_json::from_str(data).unwrap_or(serde_json::Value::Null);

        // Extract model from message_start event or default to "smart"
        let model = if event_type == "message_start" {
            event
                .get("message")
                .and_then(|m| m.get("model"))
                .and_then(|m| m.as_str())
                .unwrap_or("smart")
                .to_string()
        } else {
            "smart".to_string()
        };

        match event_type {
            "error" => Err(ApiStreamError::Unknown(format!(
                "Anthropic error: {data}"
            ))),
            "message_start" | "ping" => {
                // Skip these events, return a minimal delta
                Ok(ChatCompletionStreamResponse {
                    id: "temp".to_string(),
                    object: "chat.completion.chunk".to_string(),
                    created: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                    model,
                    choices: vec![],
                    usage: None,
                })
            }
            "content_block_start" => {
                // Handle tool_use block starts
                let content_block = event.get("content_block").ok_or_else(|| {
                    ApiStreamError::Unknown("No content_block in content_block_start".to_string())
                })?;

                let block_type = content_block
                    .get("type")
                    .and_then(|t| t.as_str())
                    .unwrap_or("");

                if block_type == "tool_use" {
                    // Extract tool call information
                    let id = content_block
                        .get("id")
                        .and_then(|i| i.as_str())
                        .unwrap_or("")
                        .to_string();
                    let name = content_block
                        .get("name")
                        .and_then(|n| n.as_str())
                        .unwrap_or("")
                        .to_string();

                    let index = event.get("index").and_then(|i| i.as_u64()).unwrap_or(0) as usize;

                    Ok(ChatCompletionStreamResponse {
                        id: "stream".to_string(),
                        object: "chat.completion.chunk".to_string(),
                        created: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs(),
                        model,
                        choices: vec![ChatCompletionStreamChoice {
                            index: 0,
                            delta: ChatMessageDelta {
                                role: Some(Role::Assistant),
                                content: None,
                                tool_calls: Some(vec![ToolCallDelta {
                                    index,
                                    id: Some(id),
                                    r#type: Some("function".to_string()),
                                    function: Some(FunctionCallDelta {
                                        name: Some(name),
                                        arguments: None,
                                    }),
                                }]),
                            },
                            finish_reason: None,
                        }],
                        usage: None,
                    })
                } else {
                    // Text block start - return minimal delta
                    Ok(ChatCompletionStreamResponse {
                        id: "stream".to_string(),
                        object: "chat.completion.chunk".to_string(),
                        created: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs(),
                        model,
                        choices: vec![ChatCompletionStreamChoice {
                            index: 0,
                            delta: ChatMessageDelta {
                                role: Some(Role::Assistant),
                                content: None,
                                tool_calls: None,
                            },
                            finish_reason: None,
                        }],
                        usage: None,
                    })
                }
            }
            "content_block_delta" => {
                let delta = event.get("delta").ok_or_else(|| {
                    ApiStreamError::Unknown("No delta in content_block_delta".to_string())
                })?;

                let delta_type = delta
                    .get("type")
                    .and_then(|t| t.as_str())
                    .unwrap_or("");

                if delta_type == "text_delta" {
                    // Text content delta
                    let content = delta.get("text").and_then(|t| t.as_str()).map(String::from);

                    Ok(ChatCompletionStreamResponse {
                        id: "stream".to_string(),
                        object: "chat.completion.chunk".to_string(),
                        created: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs(),
                        model,
                        choices: vec![ChatCompletionStreamChoice {
                            index: 0,
                            delta: ChatMessageDelta {
                                role: None,
                                content,
                                tool_calls: None,
                            },
                            finish_reason: None,
                        }],
                        usage: None,
                    })
                } else if delta_type == "input_json_delta" {
                    // Tool input delta
                    let partial_json = delta
                        .get("partial_json")
                        .and_then(|j| j.as_str())
                        .unwrap_or("")
                        .to_string();

                    let index = event.get("index").and_then(|i| i.as_u64()).unwrap_or(0) as usize;

                    Ok(ChatCompletionStreamResponse {
                        id: "stream".to_string(),
                        object: "chat.completion.chunk".to_string(),
                        created: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs(),
                        model,
                        choices: vec![ChatCompletionStreamChoice {
                            index: 0,
                            delta: ChatMessageDelta {
                                role: None,
                                content: None,
                                tool_calls: Some(vec![ToolCallDelta {
                                    index,
                                    id: None,
                                    r#type: None,
                                    function: Some(FunctionCallDelta {
                                        name: None,
                                        arguments: Some(partial_json),
                                    }),
                                }]),
                            },
                            finish_reason: None,
                        }],
                        usage: None,
                    })
                } else {
                    // Unknown delta type, return empty
                    Ok(ChatCompletionStreamResponse {
                        id: "stream".to_string(),
                        object: "chat.completion.chunk".to_string(),
                        created: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs(),
                        model,
                        choices: vec![],
                        usage: None,
                    })
                }
            }
            "message_delta" => {
                let delta_obj = event.get("delta").ok_or_else(|| {
                    ApiStreamError::Unknown("No delta in message_delta".to_string())
                })?;

                let stop_reason = delta_obj
                    .get("stop_reason")
                    .and_then(|s| s.as_str());

                let finish_reason = match stop_reason {
                    Some("end_turn") => Some(FinishReason::Stop),
                    Some("max_tokens") => Some(FinishReason::Length),
                    Some("tool_use") => Some(FinishReason::ToolCalls),
                    Some(_) | None => None,
                };

                Ok(ChatCompletionStreamResponse {
                    id: "stream".to_string(),
                    object: "chat.completion.chunk".to_string(),
                    created: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                    model,
                    choices: vec![ChatCompletionStreamChoice {
                        index: 0,
                        delta: ChatMessageDelta {
                            role: None,
                            content: None,
                            tool_calls: None,
                        },
                        finish_reason,
                    }],
                    usage: event.get("usage").and_then(|u| {
                        let input_tokens = u.get("input_tokens")?.as_u64()?.min(u32::MAX as u64) as u32;
                        let output_tokens = u.get("output_tokens")?.as_u64()?.min(u32::MAX as u64) as u32;
                        Some(Usage {
                            prompt_tokens: input_tokens,
                            completion_tokens: output_tokens,
                            total_tokens: input_tokens + output_tokens,
                            prompt_tokens_details: None,
                        })
                    }),
                })
            }
            _ => {
                // Skip unknown events
                Ok(ChatCompletionStreamResponse {
                    id: "stream".to_string(),
                    object: "chat.completion.chunk".to_string(),
                    created: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                    model,
                    choices: vec![],
                    usage: None,
                })
            }
        }
    }
}
