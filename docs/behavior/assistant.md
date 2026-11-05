# Assistant (Arcano edition)

Window > Panels > Assistant: a conversation with an AI service that
works on the open drawing. The work is in
`apps/tracedraw/src/assistente.rs`.

- At the top, the service and its model. Every text service with a key
  on the Keys page (Tools > Options > AI > Keys: Anthropic, OpenAI,
  Google Gemini, Maritaca AI, DeepSeek) is offered with its models
  (`ai::TEXT_MODELS`), and the service of Tools > Options > AI when it is
  another one (a model on this computer, an address of the user's). The
  choice is kept in the preferences (`arcano_assist`). Changing the
  service carries the conversation on in plain words (tool calls are
  kept each service's own way).
- The conversation is shown in bubbles (the user's at the right);
  markdown marks in replies are taken out. An empty conversation shows
  four suggestions that fill the field.

- The assistant has the MCP server's tools (`mcp.rs`, `mcp_plus.rs`):
  document and selection info, the scripting reference, scripts, select,
  pictures of the page or an area, font metrics, installed fonts,
  documents and windows. Each tool runs in the window, as through MCP.
- Messages API services get `tools` with `input_schema`; `tool_use`
  blocks are answered with `tool_result` blocks (text and PNG
  pictures). Chat completions services get `function` tools;
  `tool_calls` are answered with `tool` messages, and pictures follow in
  a user message (`image_url`), except for services that read no
  pictures (DeepSeek, Maritaca), which are told so in words.
- One request may make at most 40 tool calls; then it stops and says so.
  Stop ends after the step under way; New Conversation forgets it. A
  conversation never ends on an unanswered tool call.
- Every script is one undo step, as through MCP.
- Without AI features, a key or a model it says what is missing and
  sends nothing.
- Enter sends; Shift+Enter makes a new line.
