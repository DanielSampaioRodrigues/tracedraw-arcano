//! Arcano edition: the Assistant panel (Window > Panels > Assistant). A
//! conversation with the text service of Tools > Options > AI > Service
//! that works on the open drawing with the same tools as the MCP server
//! (`mcp.rs`, `mcp_plus.rs`): it reads the drawing and the selection,
//! looks at the page or a part of it, measures fonts, runs scripts,
//! selects, undoes. Each tool the service asks for runs here, in the
//! window, and its answer (text, and pictures where the service reads
//! them) goes back, until the service answers in words. Every change is
//! an undo step. See docs/behavior/assistant.md.
//!
//! Both kinds of service speak tools: the Messages API (`tool_use` and
//! `tool_result` blocks) and the chat completions API (`tool_calls` and
//! `tool` messages; a picture goes in a user message after them).

use crate::ai::Provider;
use crate::net::{HttpRequest, HttpResult};
use serde_json::{json, Value};

/// Tool calls the service may make for one request before it is stopped.
pub const MAX_STEPS: u32 = 40;
/// Tokens a reply of the Messages API may hold.
const MAX_TOKENS: u32 = 8000;
/// Longest text of a tool's answer sent back, characters.
const MAX_RESULT: usize = 60_000;

/// What the panel shows of the conversation.
#[derive(Debug, Clone, PartialEq)]
pub enum Linha {
    Voce(String),
    Assistente(String),
    /// A tool the assistant used: its name and whether it worked.
    Ferramenta { nome: String, ok: bool, resumo: String },
    Erro(String),
}

/// The conversation and the request under way.
#[derive(Default)]
pub struct Assistente {
    /// The messages in the service's own format.
    pub mensagens: Vec<Value>,
    pub linhas: Vec<Linha>,
    pub entrada: String,
    pending: Option<std::sync::mpsc::Receiver<HttpResult>>,
    /// Tool calls made for the current request.
    pub passos: u32,
    /// Stop asked: no more tool calls are answered.
    parar: bool,
    /// Answers given at once instead of the network (tests).
    pub fake: Option<fn(&HttpRequest) -> HttpResult>,
    /// Requests sent (tests look at them).
    pub enviados: Vec<HttpRequest>,
}

impl Assistente {
    pub fn ocupado(&self) -> bool {
        self.pending.is_some()
    }
}

const SYSTEM: &str = "You are the assistant inside TraceDraw, a vector drawing editor, working beside the user on the drawing open in their window. Answer in the user's language (Brazilian Portuguese unless they write otherwise), briefly and plainly. Use the tools to act: document_info and get_selection to know what is there, render_area or render_page to look, api_reference once before your first script, run_script to change the drawing (lengths in millimetres, the page's lower-left corner at 0, 0, y up; every script is one undo step and a failing script changes nothing), font_metrics before sizing or fitting text (never estimate), select to select. After changing the drawing, look at the result before saying it is done. Ask before deleting much or replacing the user's work; otherwise act. Never invent fonts: list_fonts says which are installed.";

/// The tools, as the service expects them.
fn tools(provider: Provider) -> Vec<Value> {
    crate::mcp::tool_list()
        .into_iter()
        .map(|t| {
            let name = t["name"].clone();
            let description = t["description"].clone();
            let schema = t["inputSchema"].clone();
            match provider {
                Provider::Anthropic => {
                    json!({"name": name, "description": description, "input_schema": schema})
                }
                Provider::OpenAiCompatible => json!({"type": "function", "function": {
                    "name": name, "description": description, "parameters": schema}}),
            }
        })
        .collect()
}

/// Whether the service takes pictures back (the text-only ones do not).
fn reads_pictures(prefs: &crate::ai::AiPrefs) -> bool {
    let e = prefs.endpoint().to_ascii_lowercase();
    !(crate::ai::is_maritaca(prefs) || e.contains("deepseek"))
}

/// The request for the conversation so far.
pub fn pedido(prefs: &crate::ai::AiPrefs, key: &str, mensagens: &[Value]) -> HttpRequest {
    let body = match prefs.provider {
        Provider::Anthropic => json!({
            "model": prefs.model(),
            "max_tokens": MAX_TOKENS,
            "system": SYSTEM,
            "tools": tools(prefs.provider),
            "messages": mensagens,
        }),
        Provider::OpenAiCompatible => {
            let mut all = vec![json!({"role": "system", "content": SYSTEM})];
            all.extend(mensagens.iter().cloned());
            json!({"model": prefs.model(), "messages": all, "tools": tools(prefs.provider)})
        }
    };
    HttpRequest {
        method: "POST",
        url: prefs.endpoint(),
        headers: crate::ai::headers(prefs, key),
        body: body.to_string(),
        bytes: Vec::new(),
    }
}

/// A tool the service asked for: its call id, name and arguments.
#[derive(Debug, Clone, PartialEq)]
pub struct Chamada {
    pub id: String,
    pub nome: String,
    pub args: Value,
}

/// What a reply holds: the assistant's message to keep in the
/// conversation, its words and the tools it asks for.
pub fn ler_resposta(provider: Provider, body: &Value) -> Result<(Value, String, Vec<Chamada>), String> {
    match provider {
        Provider::Anthropic => {
            let content = body.get("content").cloned().unwrap_or(json!([]));
            let blocks = content.as_array().cloned().unwrap_or_default();
            let text: String = blocks
                .iter()
                .filter(|b| b["type"] == "text")
                .filter_map(|b| b["text"].as_str())
                .collect::<Vec<_>>()
                .join("\n");
            let calls = blocks
                .iter()
                .filter(|b| b["type"] == "tool_use")
                .map(|b| Chamada {
                    id: b["id"].as_str().unwrap_or("").to_string(),
                    nome: b["name"].as_str().unwrap_or("").to_string(),
                    args: b.get("input").cloned().unwrap_or(json!({})),
                })
                .collect();
            Ok((json!({"role": "assistant", "content": content}), text, calls))
        }
        Provider::OpenAiCompatible => {
            let msg = body.pointer("/choices/0/message").cloned().ok_or_else(|| crate::i18n::tr("ai.unreadable_reply"))?;
            let text = msg["content"].as_str().unwrap_or("").to_string();
            let calls: Vec<Chamada> = msg["tool_calls"]
                .as_array()
                .map(|list| {
                    list.iter()
                        .map(|c| Chamada {
                            id: c["id"].as_str().unwrap_or("").to_string(),
                            nome: c.pointer("/function/name").and_then(|n| n.as_str()).unwrap_or("").to_string(),
                            args: c
                                .pointer("/function/arguments")
                                .and_then(|a| a.as_str())
                                .and_then(|a| serde_json::from_str(a).ok())
                                .unwrap_or(json!({})),
                        })
                        .collect()
                })
                .unwrap_or_default();
            // Kept as the service gave it (with its tool calls), content
            // never null for services that refuse that.
            let mut keep = json!({"role": "assistant", "content": text});
            if let Some(tc) = msg.get("tool_calls").filter(|t| t.as_array().is_some_and(|a| !a.is_empty())) {
                keep["tool_calls"] = tc.clone();
            }
            Ok((keep, text, calls))
        }
    }
}

/// A tool's answer: its text and its pictures (base64 PNG).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Resultado {
    pub texto: String,
    pub imagens: Vec<String>,
    pub erro: bool,
}

/// The messages that carry the answers back.
pub fn respostas(provider: Provider, pictures: bool, calls: &[(Chamada, Resultado)]) -> Vec<Value> {
    match provider {
        Provider::Anthropic => {
            let blocks: Vec<Value> = calls
                .iter()
                .map(|(c, r)| {
                    let mut content = vec![json!({"type": "text", "text": r.texto})];
                    for png in &r.imagens {
                        content.push(json!({"type": "image", "source": {"type": "base64", "media_type": "image/png", "data": png}}));
                    }
                    json!({"type": "tool_result", "tool_use_id": c.id, "content": content, "is_error": r.erro})
                })
                .collect();
            vec![json!({"role": "user", "content": blocks})]
        }
        Provider::OpenAiCompatible => {
            let mut out: Vec<Value> = calls
                .iter()
                .map(|(c, r)| {
                    let mut text = r.texto.clone();
                    if !r.imagens.is_empty() && !pictures {
                        text.push_str("\n(this service does not read pictures; describe with document_info instead)");
                    }
                    json!({"role": "tool", "tool_call_id": c.id, "content": text})
                })
                .collect();
            let pics: Vec<&String> = calls.iter().flat_map(|(_, r)| &r.imagens).collect();
            if pictures && !pics.is_empty() {
                let mut content = vec![json!({"type": "text", "text": "The pictures the tools returned:"})];
                for png in pics {
                    content.push(json!({"type": "image_url", "image_url": {"url": format!("data:image/png;base64,{png}")}}));
                }
                out.push(json!({"role": "user", "content": content}));
            }
            out
        }
    }
}

/// Run `call` on the drawing, as the MCP server would.
fn executar(app: &mut crate::app::App, call: &Chamada) -> Resultado {
    let line = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": {"name": call.nome, "arguments": call.args}})
    .to_string();
    let Some(answer) = crate::mcp::handle_line(app, &line) else {
        return Resultado { texto: "no answer".into(), erro: true, ..Default::default() };
    };
    let v: Value = serde_json::from_str(&answer).unwrap_or(json!({}));
    let result = &v["result"];
    let mut r = Resultado {
        erro: result["isError"].as_bool().unwrap_or(v.get("error").is_some()),
        ..Default::default()
    };
    if let Some(e) = v.pointer("/error/message").and_then(|m| m.as_str()) {
        r.texto = e.to_string();
    }
    for item in result["content"].as_array().cloned().unwrap_or_default() {
        match item["type"].as_str() {
            Some("text") => {
                if !r.texto.is_empty() {
                    r.texto.push('\n');
                }
                r.texto.push_str(item["text"].as_str().unwrap_or(""));
            }
            Some("image") => {
                if let Some(d) = item["data"].as_str() {
                    r.imagens.push(d.to_string());
                }
            }
            _ => {}
        }
    }
    if r.texto.chars().count() > MAX_RESULT {
        r.texto = r.texto.chars().take(MAX_RESULT).collect::<String>() + "\n(cut short)";
    }
    r
}

/// A line about a tool's answer for the panel.
fn resumo(call: &Chamada, r: &Resultado) -> String {
    let first = r.texto.lines().next().unwrap_or("").chars().take(90).collect::<String>();
    if r.erro {
        first
    } else if call.nome == "run_script" && first == "ok" {
        String::new()
    } else if !r.imagens.is_empty() {
        crate::i18n::tr("assist.looked")
    } else {
        first
    }
}

impl crate::app::App {
    /// Send what the user typed.
    pub fn assist_send(&mut self) {
        if self.assistente.ocupado() {
            return;
        }
        let text = self.assistente.entrada.trim().to_string();
        if text.is_empty() {
            return;
        }
        let prefs = self.settings.ai.clone();
        let problem = if !self.ai_features_on() {
            Some("aiimg.off")
        } else if prefs.provider.needs_key() && self.ai.key.trim().is_empty() {
            Some("ai.need_key")
        } else if prefs.model().is_empty() {
            Some("ai.need_model")
        } else {
            None
        };
        if let Some(k) = problem {
            self.assistente.linhas.push(Linha::Erro(crate::i18n::tr(k)));
            return;
        }
        self.assistente.entrada.clear();
        self.assistente.linhas.push(Linha::Voce(text.clone()));
        self.assistente.mensagens.push(json!({"role": "user", "content": text}));
        self.assistente.passos = 0;
        self.assistente.parar = false;
        self.assist_request();
    }

    fn assist_request(&mut self) {
        let http = pedido(&self.settings.ai, &self.ai.key, &self.assistente.mensagens);
        let (tx, rx) = std::sync::mpsc::channel();
        match self.assistente.fake {
            Some(fake) => {
                let _ = tx.send(fake(&http));
            }
            None => crate::net::spawn(http.clone(), 16 * 1024 * 1024, tx),
        }
        self.assistente.enviados.push(http);
        if self.assistente.enviados.len() > 4 {
            self.assistente.enviados.remove(0);
        }
        self.assistente.pending = Some(rx);
    }

    /// Stop after the step under way.
    pub fn assist_stop(&mut self) {
        self.assistente.parar = true;
        if self.assistente.pending.take().is_some() {
            self.assistente.linhas.push(Linha::Erro(crate::i18n::tr("assist.stopped")));
            self.assist_drop_open_calls();
        }
    }

    /// A new conversation.
    pub fn assist_clear(&mut self) {
        self.assistente.pending = None;
        self.assistente.mensagens.clear();
        self.assistente.linhas.clear();
        self.assistente.passos = 0;
    }

    /// The conversation must not end with tool calls that got no answer:
    /// they are taken out.
    fn assist_drop_open_calls(&mut self) {
        let open = self.assistente.mensagens.last().is_some_and(|m| {
            m["role"] == "assistant"
                && (m.get("tool_calls").is_some()
                    || m["content"].as_array().is_some_and(|c| c.iter().any(|b| b["type"] == "tool_use")))
        });
        if open {
            self.assistente.mensagens.pop();
        }
    }

    /// Take in a reply, run the tools it asks for and ask again; true
    /// while the conversation is waiting (the window keeps drawing).
    pub fn poll_assistente(&mut self) -> bool {
        let Some(rx) = &self.assistente.pending else {
            return false;
        };
        let result = match rx.try_recv() {
            Ok(r) => r,
            Err(std::sync::mpsc::TryRecvError::Empty) => return true,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => Err(crate::i18n::tr("ai.connection_lost")),
        };
        self.assistente.pending = None;
        let provider = self.settings.ai.provider;
        let parsed = result.and_then(|r| {
            let v: Value = serde_json::from_str(&r.body).map_err(|_| crate::i18n::tr("ai.unreadable_reply"))?;
            if !(200..300).contains(&r.status) {
                let m = v.pointer("/error/message").and_then(|m| m.as_str()).unwrap_or("");
                return Err(crate::i18n::trf("ai.http_error", &[("status", &r.status.to_string()), ("message", m)]));
            }
            ler_resposta(provider, &v)
        });
        let (keep, text, calls) = match parsed {
            Ok(x) => x,
            Err(e) => {
                self.assistente.linhas.push(Linha::Erro(e));
                return false;
            }
        };
        self.assistente.mensagens.push(keep);
        if !text.trim().is_empty() {
            self.assistente.linhas.push(Linha::Assistente(text.trim().to_string()));
        }
        if calls.is_empty() {
            return false;
        }
        if self.assistente.parar || self.assistente.passos >= MAX_STEPS {
            self.assist_drop_open_calls();
            if !self.assistente.parar {
                self.assistente.linhas.push(Linha::Erro(crate::i18n::trf("assist.too_many", &[("n", &MAX_STEPS.to_string())])));
            }
            return false;
        }
        let mut done = Vec::new();
        for call in calls {
            self.assistente.passos += 1;
            let r = executar(self, &call);
            self.assistente.linhas.push(Linha::Ferramenta {
                nome: call.nome.clone(),
                ok: !r.erro,
                resumo: resumo(&call, &r),
            });
            done.push((call, r));
        }
        let pictures = reads_pictures(&self.settings.ai);
        let answers = respostas(provider, pictures, &done);
        self.assistente.mensagens.extend(answers);
        self.assist_request();
        if self.assistente.fake.is_some() {
            return self.poll_assistente();
        }
        true
    }
}

/// The panel: the conversation, then the field, Send, Stop and New.
pub fn docker(app: &mut crate::app::App, ui: &mut egui::Ui) {
    use crate::i18n::tr;
    use crate::ui::tips::RichTip as _;
    let busy = app.assistente.ocupado();
    let service = crate::i18n::trf("assist.service", &[("model", &app.settings.ai.model())]);
    ui.label(egui::RichText::new(service).color(crate::theme::Tokens::text_dim()))
        .rich_tip(crate::ui::tips::pair("assist.service_tip", "assist.service_tip_desc"));
    let bottom = 96.0;
    egui::ScrollArea::vertical()
        .max_height((ui.available_height() - bottom).max(80.0))
        .stick_to_bottom(true)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            if app.assistente.linhas.is_empty() {
                ui.label(egui::RichText::new(tr("assist.empty")).color(crate::theme::Tokens::text_dim()));
            }
            for l in &app.assistente.linhas {
                match l {
                    Linha::Voce(t) => {
                        ui.label(egui::RichText::new(t).strong());
                    }
                    Linha::Assistente(t) => {
                        ui.label(t);
                    }
                    Linha::Ferramenta { nome, ok, resumo } => {
                        let mark = if *ok { "\u{2713}" } else { "\u{2717}" };
                        let line = if resumo.is_empty() { format!("{mark} {nome}") } else { format!("{mark} {nome}: {resumo}") };
                        ui.label(egui::RichText::new(line).small().color(crate::theme::Tokens::text_dim()));
                    }
                    Linha::Erro(t) => {
                        ui.label(egui::RichText::new(t).color(egui::Color32::from_rgb(220, 90, 70)));
                    }
                }
                ui.add_space(4.0);
            }
            if busy {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(tr("assist.working"));
                });
            }
        });
    ui.separator();
    let field = ui.add(
        egui::TextEdit::multiline(&mut app.assistente.entrada)
            .desired_rows(3)
            .desired_width(f32::INFINITY)
            .hint_text(tr("assist.hint")),
    );
    let field = field.rich_tip(crate::ui::tips::pair("assist.field", "assist.field_desc"));
    // Ctrl+Enter sends.
    let send_key = field.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter) && i.modifiers.command);
    ui.horizontal(|ui| {
        if ui
            .add_enabled(!busy, egui::Button::new(tr("assist.send")))
            .rich_tip(crate::ui::tips::pair("assist.send", "assist.send_desc"))
            .clicked()
            || (send_key && !busy)
        {
            app.assist_send();
        }
        if ui
            .add_enabled(busy, egui::Button::new(tr("assist.stop")))
            .rich_tip(crate::ui::tips::pair("assist.stop", "assist.stop_desc"))
            .clicked()
        {
            app.assist_stop();
        }
        if ui
            .button(tr("assist.new"))
            .rich_tip(crate::ui::tips::pair("assist.new", "assist.new_desc"))
            .clicked()
        {
            app.assist_clear();
        }
    });
    if busy {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(150));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use crate::net::HttpResponse;

    fn ready(app: &mut App, provider: Provider) {
        app.settings.ai_enabled = true;
        app.settings.ai.provider = provider;
        app.settings.ai.model = "model-x".into();
        app.ai.key = "k".into();
    }

    /// A service that first asks to draw a circle, then says it is done.
    fn anthropic(req: &HttpRequest) -> HttpResult {
        let body: Value = serde_json::from_str(&req.body).unwrap();
        let last = body["messages"].as_array().unwrap().last().unwrap().clone();
        let answered = last["content"].as_array().is_some_and(|c| c.iter().any(|b| b["type"] == "tool_result"));
        let reply = if answered {
            json!({"content": [{"type": "text", "text": "Pronto: desenhei o círculo."}], "stop_reason": "end_turn"})
        } else {
            json!({"content": [
                {"type": "text", "text": "Vou desenhar."},
                {"type": "tool_use", "id": "t1", "name": "run_script", "input": {"source": "ActiveLayer.CreateEllipse(10, 10, 20, 20); print('ok')"}}
            ], "stop_reason": "tool_use"})
        };
        Ok(HttpResponse { status: 200, body: reply.to_string() })
    }

    fn openai(req: &HttpRequest) -> HttpResult {
        let body: Value = serde_json::from_str(&req.body).unwrap();
        assert_eq!(body["messages"][0]["role"], "system");
        assert!(body["tools"].as_array().is_some_and(|t| t.iter().any(|t| t["function"]["name"] == "render_area")));
        let msgs = body["messages"].as_array().unwrap();
        let answered = msgs.iter().any(|m| m["role"] == "tool");
        let reply = if answered {
            // The picture came back after the tool's answer.
            assert!(msgs.iter().any(|m| m["role"] == "user" && m["content"].is_array()));
            json!({"choices": [{"message": {"role": "assistant", "content": "A página está vazia."}, "finish_reason": "stop"}]})
        } else {
            json!({"choices": [{"message": {"role": "assistant", "content": null, "tool_calls": [
                {"id": "c1", "type": "function", "function": {"name": "render_area", "arguments": "{\"area\": \"page\", \"max_px\": 128}"}}
            ]}, "finish_reason": "tool_calls"}]})
        };
        Ok(HttpResponse { status: 200, body: reply.to_string() })
    }

    #[test]
    fn the_assistant_draws_with_the_tools_and_answers() {
        let mut app = App::headless();
        ready(&mut app, Provider::Anthropic);
        app.assistente.fake = Some(anthropic);
        let before = app.doc().pages[0].layers.iter().map(|l| l.shapes.len()).sum::<usize>();
        app.assistente.entrada = "desenhe um círculo".into();
        app.assist_send();
        app.poll_assistente();
        let after = app.doc().pages[0].layers.iter().map(|l| l.shapes.len()).sum::<usize>();
        assert_eq!(after, before + 1, "the circle was drawn");
        assert!(matches!(app.assistente.linhas.last(), Some(Linha::Assistente(t)) if t.contains("Pronto")));
        assert!(app.assistente.linhas.iter().any(|l| matches!(l, Linha::Ferramenta { nome, ok: true, .. } if nome == "run_script")));
        // The tools went with the request, in the Messages API's form.
        let sent: Value = serde_json::from_str(&app.assistente.enviados[0].body).unwrap();
        assert!(sent["tools"].as_array().is_some_and(|t| t.iter().any(|t| t["name"] == "get_selection" && t["input_schema"].is_object())));
        // One undo step takes it back.
        app.undo();
        assert_eq!(app.doc().pages[0].layers.iter().map(|l| l.shapes.len()).sum::<usize>(), before);
    }

    #[test]
    fn a_chat_completions_service_looks_at_the_page() {
        let mut app = App::headless();
        ready(&mut app, Provider::OpenAiCompatible);
        app.settings.ai.endpoint = "https://example.org/v1/chat/completions".into();
        app.assistente.fake = Some(openai);
        app.assistente.entrada = "o que tem na página?".into();
        app.assist_send();
        app.poll_assistente();
        assert!(matches!(app.assistente.linhas.last(), Some(Linha::Assistente(t)) if t.contains("vazia")), "{:?}", app.assistente.linhas);
    }

    #[test]
    fn without_a_key_it_says_so() {
        let mut app = App::headless();
        app.settings.ai_enabled = true;
        app.settings.ai.provider = Provider::Anthropic;
        app.ai.key.clear();
        app.assistente.entrada = "oi".into();
        app.assist_send();
        assert!(matches!(app.assistente.linhas.last(), Some(Linha::Erro(_))));
        assert!(app.assistente.enviados.is_empty());
    }

    #[test]
    fn a_service_that_never_stops_is_stopped() {
        fn forever(_: &HttpRequest) -> HttpResult {
            let reply = json!({"content": [{"type": "tool_use", "id": "t", "name": "document_info", "input": {}}], "stop_reason": "tool_use"});
            Ok(HttpResponse { status: 200, body: reply.to_string() })
        }
        let mut app = App::headless();
        ready(&mut app, Provider::Anthropic);
        app.assistente.fake = Some(forever);
        app.assistente.entrada = "x".into();
        app.assist_send();
        app.poll_assistente();
        assert_eq!(app.assistente.passos, MAX_STEPS);
        assert!(matches!(app.assistente.linhas.last(), Some(Linha::Erro(_))));
        // The conversation does not end with an unanswered call.
        let last = app.assistente.mensagens.last().unwrap();
        assert_eq!(last["role"], "user");
    }
}
