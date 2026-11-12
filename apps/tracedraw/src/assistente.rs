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
    Ferramenta {
        nome: String,
        ok: bool,
        resumo: String,
    },
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
    /// The connection and key of the request under way.
    pub conn: Option<(crate::ai::AiPrefs, String)>,
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
pub fn ler_resposta(
    provider: Provider,
    body: &Value,
) -> Result<(Value, String, Vec<Chamada>), String> {
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
            Ok((
                json!({"role": "assistant", "content": content}),
                text,
                calls,
            ))
        }
        Provider::OpenAiCompatible => {
            let msg = body
                .pointer("/choices/0/message")
                .cloned()
                .ok_or_else(|| crate::i18n::tr("ai.unreadable_reply"))?;
            let text = msg["content"].as_str().unwrap_or("").to_string();
            let calls: Vec<Chamada> = msg["tool_calls"]
                .as_array()
                .map(|list| {
                    list.iter()
                        .map(|c| Chamada {
                            id: c["id"].as_str().unwrap_or("").to_string(),
                            nome: c
                                .pointer("/function/name")
                                .and_then(|n| n.as_str())
                                .unwrap_or("")
                                .to_string(),
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
            if let Some(tc) = msg
                .get("tool_calls")
                .filter(|t| t.as_array().is_some_and(|a| !a.is_empty()))
            {
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
                let mut content =
                    vec![json!({"type": "text", "text": "The pictures the tools returned:"})];
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
        return Resultado {
            texto: "no answer".into(),
            erro: true,
            ..Default::default()
        };
    };
    let v: Value = serde_json::from_str(&answer).unwrap_or(json!({}));
    let result = &v["result"];
    let mut r = Resultado {
        erro: result["isError"]
            .as_bool()
            .unwrap_or(v.get("error").is_some()),
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
    let first = r
        .texto
        .lines()
        .next()
        .unwrap_or("")
        .chars()
        .take(90)
        .collect::<String>();
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

/// The service and model chosen in the panel, kept with the preferences.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct AssistPrefs {
    /// The preset's key (`aipreset.anthropic`...), or `main` for the
    /// service of Tools > Options > AI when it is none of them.
    pub servico: String,
    pub modelo: String,
}

/// A service the assistant can talk to: one with a key on the Keys page
/// (Tools > Options > AI > Keys), or the main AI service.
#[derive(Debug, Clone)]
pub struct Conexao {
    pub id: String,
    pub nome: String,
    pub prefs: crate::ai::AiPrefs,
    pub key: String,
    pub modelos: Vec<String>,
}

/// The services the assistant can use now: every text service with a
/// key, then the main service when it is another one (a model on this
/// computer, an address of the user's).
pub fn conexoes(app: &crate::app::App) -> Vec<Conexao> {
    use crate::ai::{AiPrefs, PRESETS, TEXT_MODELS};
    let mut out = Vec::new();
    for (preset, provider, endpoint) in PRESETS {
        let Some(id) = crate::studio::provider_of_preset(preset) else {
            continue;
        };
        let key = app.studio_key(id);
        if key.trim().is_empty() {
            continue;
        }
        let modelos: Vec<String> = TEXT_MODELS
            .iter()
            .filter(|(k, ..)| *k == preset)
            .map(|(_, m, ..)| m.to_string())
            .collect();
        out.push(Conexao {
            id: preset.to_string(),
            nome: crate::i18n::tr(preset),
            prefs: AiPrefs {
                provider,
                endpoint: endpoint.to_string(),
                ..AiPrefs::default()
            },
            key,
            modelos,
        });
    }
    let main = &app.settings.ai;
    let listed = crate::ai::preset_of(main)
        .and_then(|i| PRESETS.get(i))
        .is_some_and(|(k, ..)| out.iter().any(|c| c.id == *k));
    let ready = !main.provider.needs_key() || !app.ai.key.trim().is_empty();
    if !listed && ready && !main.model().is_empty() {
        let nome = match crate::ai::preset_of(main).and_then(|i| PRESETS.get(i)) {
            Some((k, ..)) => crate::i18n::tr(k),
            None => crate::i18n::tr("assist.main_service"),
        };
        let mut modelos = vec![main.model()];
        for m in crate::ai::preset_text_models(main) {
            if !modelos.iter().any(|x| x == m) {
                modelos.push(m.to_string());
            }
        }
        out.push(Conexao {
            id: "main".into(),
            nome,
            prefs: main.clone(),
            key: app.ai.key.clone(),
            modelos,
        });
    }
    out
}

/// The service and model in use: the chosen ones, else the first service
/// and its first model.
pub fn ativa(app: &crate::app::App) -> Option<(Conexao, String)> {
    let list = conexoes(app);
    let chosen = &app.settings.arcano_assist;
    let c = list
        .iter()
        .find(|c| c.id == chosen.servico)
        .or(list.first())?
        .clone();
    let modelo = if c.modelos.iter().any(|m| *m == chosen.modelo) {
        chosen.modelo.clone()
    } else {
        c.modelos.first().cloned().unwrap_or_default()
    };
    Some((c, modelo))
}

/// The conversation as plain words, alternating, for another service
/// (each kind of service keeps tool calls its own way).
fn em_palavras(linhas: &[Linha]) -> Vec<Value> {
    let mut out: Vec<Value> = Vec::new();
    for l in linhas {
        let (role, text) = match l {
            Linha::Voce(t) => ("user", t),
            Linha::Assistente(t) => ("assistant", t),
            _ => continue,
        };
        match out.last_mut() {
            Some(last) if last["role"] == role => {
                let joined = format!("{}\n\n{}", last["content"].as_str().unwrap_or(""), text);
                last["content"] = json!(joined);
            }
            _ => out.push(json!({"role": role, "content": text})),
        }
    }
    // A conversation starts with the user.
    while out.first().is_some_and(|m| m["role"] != "user") {
        out.remove(0);
    }
    out
}

impl crate::app::App {
    /// Use `servico` and `modelo` from the next message on.
    pub fn assist_choose(&mut self, servico: String, modelo: String) {
        let before = self.settings.arcano_assist.servico.clone();
        if servico != before && !self.assistente.mensagens.is_empty() {
            self.assistente.mensagens = em_palavras(&self.assistente.linhas);
        }
        self.settings.arcano_assist = AssistPrefs { servico, modelo };
        self.settings.save();
    }

    /// Send what the user typed.
    pub fn assist_send(&mut self) {
        if self.assistente.ocupado() {
            return;
        }
        let text = self.assistente.entrada.trim().to_string();
        if text.is_empty() {
            return;
        }
        let problem = if !self.ai_features_on() {
            Some("aiimg.off")
        } else {
            match ativa(self) {
                None => Some("assist.no_service"),
                Some((_, m)) if m.is_empty() => Some("ai.need_model"),
                Some((c, m)) => {
                    let mut prefs = c.prefs.clone();
                    prefs.model = m;
                    self.assistente.conn = Some((prefs, c.key.clone()));
                    None
                }
            }
        };
        if let Some(k) = problem {
            self.assistente.linhas.push(Linha::Erro(crate::i18n::tr(k)));
            return;
        }
        self.assistente.entrada.clear();
        self.assistente.linhas.push(Linha::Voce(text.clone()));
        self.assistente
            .mensagens
            .push(json!({"role": "user", "content": text}));
        self.assistente.passos = 0;
        self.assistente.parar = false;
        self.assist_request();
    }

    fn assist_request(&mut self) {
        let Some((prefs, key)) = self.assistente.conn.clone() else {
            return;
        };
        let http = pedido(&prefs, &key, &self.assistente.mensagens);
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
            self.assistente
                .linhas
                .push(Linha::Erro(crate::i18n::tr("assist.stopped")));
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
                    || m["content"]
                        .as_array()
                        .is_some_and(|c| c.iter().any(|b| b["type"] == "tool_use")))
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
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                Err(crate::i18n::tr("ai.connection_lost"))
            }
        };
        self.assistente.pending = None;
        let Some((prefs, _)) = self.assistente.conn.clone() else {
            return false;
        };
        let provider = prefs.provider;
        let parsed = result.and_then(|r| {
            let v: Value = serde_json::from_str(&r.body)
                .map_err(|_| crate::i18n::tr("ai.unreadable_reply"))?;
            if !(200..300).contains(&r.status) {
                let m = v
                    .pointer("/error/message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("");
                return Err(crate::i18n::trf(
                    "ai.http_error",
                    &[("status", &r.status.to_string()), ("message", m)],
                ));
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
            self.assistente
                .linhas
                .push(Linha::Assistente(text.trim().to_string()));
        }
        if calls.is_empty() {
            return false;
        }
        if self.assistente.parar || self.assistente.passos >= MAX_STEPS {
            self.assist_drop_open_calls();
            if !self.assistente.parar {
                self.assistente.linhas.push(Linha::Erro(crate::i18n::trf(
                    "assist.too_many",
                    &[("n", &MAX_STEPS.to_string())],
                )));
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
        let pictures = reads_pictures(&prefs);
        let answers = respostas(provider, pictures, &done);
        self.assistente.mensagens.extend(answers);
        self.assist_request();
        if self.assistente.fake.is_some() {
            return self.poll_assistente();
        }
        true
    }
}

/// A reply's text for the panel: markdown's marks taken out (the panel
/// shows plain text), list dashes as bullets.
pub fn sem_marcas(text: &str) -> String {
    text.lines()
        .map(|l| {
            let t = l.trim_start();
            let indent = &l[..l.len() - t.len()];
            let t = t.trim_start_matches('#').trim_start();
            let t = match t.strip_prefix("- ").or_else(|| t.strip_prefix("* ")) {
                Some(rest) => format!("\u{2022} {rest}"),
                None => t.to_string(),
            };
            format!("{indent}{}", t.replace("**", "").replace('`', ""))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Suggestions shown on an empty conversation (i18n keys).
const SUGESTOES: [&str; 4] = [
    "assist.suggest_1",
    "assist.suggest_2",
    "assist.suggest_3",
    "assist.suggest_4",
];

/// One message as a bubble: the user's at the right in the accent colour,
/// the assistant's at the left.
fn bolha(ui: &mut egui::Ui, text: &str, mine: bool) {
    use crate::theme::Tokens;
    let max = (ui.available_width() * 0.86).max(120.0);
    let layout = if mine {
        egui::Layout::right_to_left(egui::Align::Min)
    } else {
        egui::Layout::left_to_right(egui::Align::Min)
    };
    ui.with_layout(layout, |ui| {
        let (fill, color) = if mine {
            (Tokens::accent(), egui::Color32::WHITE)
        } else {
            (Tokens::field(), Tokens::text())
        };
        egui::Frame::new()
            .fill(fill)
            .stroke(if mine {
                egui::Stroke::NONE
            } else {
                egui::Stroke::new(1.0, Tokens::border())
            })
            .corner_radius(10)
            .inner_margin(egui::Margin::symmetric(10, 7))
            .show(ui, |ui| {
                ui.set_max_width(max);
                ui.vertical(|ui| {
                    ui.add(
                        egui::Label::new(egui::RichText::new(text).color(color))
                            .wrap()
                            .selectable(true),
                    );
                });
            });
    });
}

/// The panel: the service and model at the top, the conversation in
/// bubbles, and the field at the bottom (Enter sends, Shift+Enter makes
/// a new line).
pub fn docker(app: &mut crate::app::App, ui: &mut egui::Ui) {
    use crate::i18n::tr;
    use crate::theme::Tokens;
    use crate::ui::tips::{pair, RichTip as _};
    let busy = app.assistente.ocupado();
    let list = conexoes(app);
    let current = ativa(app);

    // The service and its model.
    ui.horizontal_wrapped(|ui| {
        match &current {
            Some((c, modelo)) => {
                let mut servico = c.id.clone();
                ui.add_enabled_ui(!busy, |ui| {
                    crate::ui::combo::from_id_salt("assist_service")
                        .width(150.0)
                        .selected_text(&c.nome)
                        .show_ui(ui, |ui| {
                            for o in &list {
                                ui.selectable_value(&mut servico, o.id.clone(), &o.nome);
                            }
                        })
                        .response
                        .rich_tip(pair("assist.pick_service", "assist.pick_service_desc"));
                });
                if servico != c.id {
                    let first = list
                        .iter()
                        .find(|o| o.id == servico)
                        .and_then(|o| o.modelos.first().cloned())
                        .unwrap_or_default();
                    app.assist_choose(servico, first);
                } else {
                    let mut m = modelo.clone();
                    ui.add_enabled_ui(!busy, |ui| {
                        crate::ui::combo::from_id_salt("assist_model")
                            .width(150.0)
                            .selected_text(&m)
                            .show_ui(ui, |ui| {
                                for o in &c.modelos {
                                    ui.selectable_value(&mut m, o.clone(), o);
                                }
                            })
                            .response
                            .rich_tip(pair("assist.pick_model", "assist.pick_model_desc"));
                    });
                    if m != *modelo {
                        app.assist_choose(c.id.clone(), m);
                    }
                }
            }
            None => {
                ui.label(egui::RichText::new(tr("assist.no_service")).color(Tokens::text_dim()))
                    .rich_tip(pair("assist.service_tip", "assist.service_tip_desc"));
            }
        }
        if !app.assistente.linhas.is_empty()
            && ui
                .add_enabled(!busy, egui::Button::new(tr("assist.new")).small())
                .rich_tip(pair("assist.new", "assist.new_desc"))
                .clicked()
        {
            app.assist_clear();
        }
    });
    ui.add_space(4.0);

    // The conversation.
    let bottom = 118.0;
    egui::ScrollArea::vertical()
        .max_height((ui.available_height() - bottom).max(80.0))
        .stick_to_bottom(true)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 8.0;
            if app.assistente.linhas.is_empty() {
                ui.add_space(12.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new(tr("assist.welcome"))
                            .size(16.0)
                            .strong(),
                    );
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(tr("assist.empty")).color(Tokens::text_dim()),
                        )
                        .wrap(),
                    );
                });
                ui.add_space(8.0);
                for k in SUGESTOES {
                    let text = tr(k);
                    let b = egui::Button::new(egui::RichText::new(&text))
                        .corner_radius(14)
                        .wrap();
                    if ui
                        .add_enabled(!busy, b)
                        .rich_tip(pair("assist.suggest", "assist.suggest_desc"))
                        .clicked()
                    {
                        app.assistente.entrada = text;
                    }
                }
            }
            for l in &app.assistente.linhas {
                match l {
                    Linha::Voce(t) => bolha(ui, t, true),
                    Linha::Assistente(t) => bolha(ui, &sem_marcas(t), false),
                    Linha::Ferramenta { nome, ok, resumo } => {
                        let mark = if *ok { "\u{203A}" } else { "!" };
                        let line = if resumo.is_empty() {
                            format!("{mark} {nome}")
                        } else {
                            format!("{mark} {nome}: {resumo}")
                        };
                        let color = if *ok {
                            Tokens::text_dim()
                        } else {
                            egui::Color32::from_rgb(220, 140, 40)
                        };
                        ui.horizontal(|ui| {
                            ui.add_space(6.0);
                            ui.add(
                                egui::Label::new(egui::RichText::new(line).small().color(color))
                                    .truncate(),
                            );
                        });
                    }
                    Linha::Erro(t) => {
                        egui::Frame::new()
                            .fill(egui::Color32::from_rgba_unmultiplied(220, 90, 70, 30))
                            .corner_radius(8)
                            .inner_margin(egui::Margin::symmetric(10, 6))
                            .show(ui, |ui| {
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(t)
                                            .color(egui::Color32::from_rgb(230, 100, 80)),
                                    )
                                    .wrap(),
                                );
                            });
                    }
                }
            }
            if busy {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(egui::RichText::new(tr("assist.working")).color(Tokens::text_dim()));
                });
            }
        });

    // The field: Enter sends, Shift+Enter makes a new line. The Enter is
    // taken before the field sees it.
    let input_id = egui::Id::new("arcano_assist_input");
    let focused = ui.memory(|m| m.has_focus(input_id));
    let enter = focused
        && !busy
        && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
    ui.add_space(6.0);
    egui::Frame::new()
        .fill(Tokens::field())
        .stroke(egui::Stroke::new(
            1.0,
            if focused {
                Tokens::accent()
            } else {
                Tokens::control_border()
            },
        ))
        .corner_radius(10)
        .inner_margin(egui::Margin::symmetric(8, 6))
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut app.assistente.entrada)
                    .id(input_id)
                    .frame(egui::Frame::NONE)
                    .desired_rows(3)
                    .desired_width(f32::INFINITY)
                    .hint_text(tr("assist.hint")),
            )
            .rich_tip(pair("assist.field", "assist.field_desc"));
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(tr("assist.keys_hint"))
                        .small()
                        .color(Tokens::text_dim()),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if busy {
                        if ui
                            .button(tr("assist.stop"))
                            .rich_tip(pair("assist.stop", "assist.stop_desc"))
                            .clicked()
                        {
                            app.assist_stop();
                        }
                    } else {
                        let ready = !app.assistente.entrada.trim().is_empty();
                        let send = egui::Button::new(
                            egui::RichText::new(tr("assist.send")).color(egui::Color32::WHITE),
                        )
                        .fill(Tokens::accent())
                        .corner_radius(6);
                        if ui
                            .add_enabled(ready, send)
                            .rich_tip(pair("assist.send", "assist.send_desc"))
                            .clicked()
                        {
                            app.assist_send();
                        }
                    }
                });
            });
        });
    if enter {
        app.assist_send();
        ui.memory_mut(|m| m.request_focus(input_id));
    }
    if busy {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(150));
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
        let answered = last["content"]
            .as_array()
            .is_some_and(|c| c.iter().any(|b| b["type"] == "tool_result"));
        let reply = if answered {
            json!({"content": [{"type": "text", "text": "Pronto: desenhei o círculo."}], "stop_reason": "end_turn"})
        } else {
            json!({"content": [
                {"type": "text", "text": "Vou desenhar."},
                {"type": "tool_use", "id": "t1", "name": "run_script", "input": {"source": "ActiveLayer.CreateEllipse(10, 10, 20, 20); print('ok')"}}
            ], "stop_reason": "tool_use"})
        };
        Ok(HttpResponse {
            status: 200,
            body: reply.to_string(),
            retry_after: None,
        })
    }

    fn openai(req: &HttpRequest) -> HttpResult {
        let body: Value = serde_json::from_str(&req.body).unwrap();
        assert_eq!(body["messages"][0]["role"], "system");
        assert!(body["tools"]
            .as_array()
            .is_some_and(|t| t.iter().any(|t| t["function"]["name"] == "render_area")));
        let msgs = body["messages"].as_array().unwrap();
        let answered = msgs.iter().any(|m| m["role"] == "tool");
        let reply = if answered {
            // The picture came back after the tool's answer.
            assert!(msgs
                .iter()
                .any(|m| m["role"] == "user" && m["content"].is_array()));
            json!({"choices": [{"message": {"role": "assistant", "content": "A página está vazia."}, "finish_reason": "stop"}]})
        } else {
            json!({"choices": [{"message": {"role": "assistant", "content": null, "tool_calls": [
                {"id": "c1", "type": "function", "function": {"name": "render_area", "arguments": "{\"area\": \"page\", \"max_px\": 128}"}}
            ]}, "finish_reason": "tool_calls"}]})
        };
        Ok(HttpResponse {
            status: 200,
            body: reply.to_string(),
            retry_after: None,
        })
    }

    #[test]
    fn the_assistant_draws_with_the_tools_and_answers() {
        let mut app = App::headless();
        ready(&mut app, Provider::Anthropic);
        app.assistente.fake = Some(anthropic);
        let before = app.doc().pages[0]
            .layers
            .iter()
            .map(|l| l.shapes.len())
            .sum::<usize>();
        app.assistente.entrada = "desenhe um círculo".into();
        app.assist_send();
        app.poll_assistente();
        let after = app.doc().pages[0]
            .layers
            .iter()
            .map(|l| l.shapes.len())
            .sum::<usize>();
        assert_eq!(after, before + 1, "the circle was drawn");
        assert!(
            matches!(app.assistente.linhas.last(), Some(Linha::Assistente(t)) if t.contains("Pronto"))
        );
        assert!(app.assistente.linhas.iter().any(
            |l| matches!(l, Linha::Ferramenta { nome, ok: true, .. } if nome == "run_script")
        ));
        // The tools went with the request, in the Messages API's form.
        let sent: Value = serde_json::from_str(&app.assistente.enviados[0].body).unwrap();
        assert!(sent["tools"].as_array().is_some_and(|t| t
            .iter()
            .any(|t| t["name"] == "get_selection" && t["input_schema"].is_object())));
        // One undo step takes it back.
        app.undo();
        assert_eq!(
            app.doc().pages[0]
                .layers
                .iter()
                .map(|l| l.shapes.len())
                .sum::<usize>(),
            before
        );
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
        assert!(
            matches!(app.assistente.linhas.last(), Some(Linha::Assistente(t)) if t.contains("vazia")),
            "{:?}",
            app.assistente.linhas
        );
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
            Ok(HttpResponse {
                status: 200,
                body: reply.to_string(),
                retry_after: None,
            })
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

    #[test]
    fn every_service_with_a_key_is_offered_with_its_models() {
        let mut app = App::headless();
        app.settings.ai_enabled = true;
        app.studio_set_key("anthropic", "ka".into());
        app.studio_set_key("openai", "ko".into());
        let list = conexoes(&app);
        let ids: Vec<&str> = list.iter().map(|c| c.id.as_str()).collect();
        assert!(
            ids.contains(&"aipreset.anthropic") && ids.contains(&"aipreset.openai"),
            "{ids:?}"
        );
        let claude = list.iter().find(|c| c.id == "aipreset.anthropic").unwrap();
        assert!(
            claude.modelos.iter().all(|m| m.starts_with("claude-")),
            "{:?}",
            claude.modelos
        );
        // The chosen one is used, with its own key and model.
        app.assist_choose("aipreset.openai".into(), "gpt-6-luna".into());
        fn ok(req: &HttpRequest) -> HttpResult {
            let body: Value = serde_json::from_str(&req.body).unwrap();
            assert_eq!(body["model"], "gpt-6-luna");
            assert!(req.headers.iter().any(|(_, v)| v.contains("ko")));
            let reply = json!({"choices": [{"message": {"role": "assistant", "content": "oi"}}]});
            Ok(HttpResponse {
                status: 200,
                body: reply.to_string(),
                retry_after: None,
            })
        }
        app.assistente.fake = Some(ok);
        app.assistente.entrada = "oi".into();
        app.assist_send();
        app.poll_assistente();
        assert!(
            matches!(app.assistente.linhas.last(), Some(Linha::Assistente(t)) if t == "oi"),
            "{:?}",
            app.assistente.linhas
        );
        // Another service carries on the conversation in plain words.
        app.assist_choose("aipreset.anthropic".into(), "claude-opus-5-5".into());
        assert_eq!(app.assistente.mensagens.len(), 2);
        assert_eq!(app.assistente.mensagens[0]["role"], "user");
        assert_eq!(app.assistente.mensagens[1]["content"], "oi");
    }

    #[test]
    fn markdown_marks_are_taken_out() {
        assert_eq!(
            sem_marcas("## Pronto\n- **um**\n- `dois`"),
            "Pronto\n\u{2022} um\n\u{2022} dois"
        );
    }

    /// The panel keeps the docker's width, with a conversation and a
    /// suggestion list alike.
    #[test]
    fn the_panel_keeps_its_width() {
        let mut app = App::headless();
        ready(&mut app, Provider::Anthropic);
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1400.0, 800.0));
        let mut widths = Vec::new();
        for frame in 0..10 {
            if frame == 5 {
                app.assistente
                    .linhas
                    .push(Linha::Voce("um pedido bem comprido ".repeat(20)));
                app.assistente
                    .linhas
                    .push(Linha::Assistente("uma resposta **longa** ".repeat(30)));
            }
            let mut o = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(screen),
                    ..Default::default()
                },
                |ui| {
                    let r = egui::Panel::right("dock")
                        .default_size(300.0)
                        .show(ui, |ui| docker(&mut app, ui));
                    widths.push(r.response.rect.width());
                },
            );
            o.textures_delta.clear();
        }
        assert!(widths.iter().all(|w| *w < 400.0), "{widths:?}");
        assert!((widths[9] - widths[6]).abs() < 1.0, "{widths:?}");
    }
}
