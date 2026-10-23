//! The studio's catalog: the services, their models and operations, the
//! sizes and shapes each model makes and the listed prices. It is data,
//! not code, so that a new model or a new price reaches every computer
//! without a new build: the copy built in (`assets/arcano/ai-catalog.json`)
//! is replaced by a newer one from the edition's repository, fetched at
//! most once a day and kept in the preferences folder.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::Op;

/// Text in several languages (`"en"`, `"pt-BR"`...); English when the
/// interface language has none.
pub type Text = BTreeMap<String, String>;

/// The text for the interface language.
pub fn text(t: &Text) -> String {
    let lang = crate::i18n::language();
    t.get(&lang)
        .or_else(|| t.get("en"))
        .or_else(|| t.values().next())
        .cloned()
        .unwrap_or_default()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Catalog {
    /// Increases with each published catalog; a fetched one replaces the
    /// built-in one only when newer.
    pub version: u32,
    /// When the prices were checked, `YYYY-MM-DD`.
    pub checked: String,
    pub providers: Vec<ProviderInfo>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ProviderInfo {
    /// `gemini`, `openai`, `recraft`, `freepik`, `stability`.
    pub id: String,
    pub name: String,
    /// Where keys are made.
    pub key_url: String,
    pub about: Text,
    /// The service takes words in English only.
    pub english_only: bool,
    /// Picture models (Generate and Remix).
    pub models: Vec<ModelInfo>,
    /// Operations on a picture, with their variants.
    pub ops: Vec<OpInfo>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ModelInfo {
    pub id: String,
    pub label: String,
    pub note: Text,
    /// `generate`, `remix`.
    pub ops: Vec<String>,
    /// The shapes it makes, `"1:1"`...
    pub aspects: Vec<String>,
    /// The sizes it makes, `"1K"`, `"2K"`, `"4K"`.
    pub resolutions: Vec<String>,
    /// Listed price of one picture in US dollars, by size; a size missing
    /// from the map has no listed price.
    pub price: BTreeMap<String, f64>,
    /// When the price depends on more than the size: the lowest and the
    /// highest.
    pub price_range: Option<(f64, f64)>,
    /// A note on the price (credits, quality...).
    pub price_note: Text,
    /// It makes vectors (SVG) rather than pixels.
    pub vector: bool,
    /// It is the service's newest; offered first.
    pub newest: bool,
    /// Choices the model takes (quality, how much a remix changes...).
    pub params: Vec<Param>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct OpInfo {
    /// `upscale`, `expand`... (`Op::id`).
    pub op: String,
    pub variants: Vec<Variant>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Variant {
    pub id: String,
    pub label: Text,
    pub note: Text,
    /// Listed price of one run in US dollars.
    pub price: Option<f64>,
    pub price_note: Text,
    /// Choices the variant takes (`factor` 2 or 4...).
    pub params: Vec<Param>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Param {
    pub name: String,
    pub label: Text,
    /// `choice`, `number`, `text`.
    pub kind: String,
    /// For `choice`: the values, as the service names them, with labels.
    pub choices: Vec<(serde_json::Value, Text)>,
    pub default: serde_json::Value,
    pub min: Option<f64>,
    pub max: Option<f64>,
}

/// The catalog built into this build.
pub const BUILT_IN: &str = include_str!("../../../../assets/arcano/ai-catalog.json");

/// Where a newer catalog is published.
pub const REMOTE_URL: &str = concat!(
    "https://raw.githubusercontent.com/",
    fork_repo!(),
    "/arcano/assets/arcano/ai-catalog.json"
);

/// The file a fetched catalog is kept in, in the preferences folder.
pub const CACHE_FILE: &str = "ai-catalog.json";

impl Catalog {
    /// The built-in catalog (an empty one if it does not read, which a
    /// test rules out).
    pub fn built_in() -> Catalog {
        serde_json::from_str(BUILT_IN).unwrap_or_default()
    }

    /// The best catalog at hand: the kept one when it is newer than the
    /// built-in one.
    pub fn load() -> Catalog {
        let built = Catalog::built_in();
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(kept) = crate::settings::config_dir()
            .map(|d| d.join(CACHE_FILE))
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str::<Catalog>(&s).ok())
        {
            if kept.version > built.version && !kept.providers.is_empty() {
                return kept;
            }
        }
        built
    }

    /// Read a fetched catalog; `Some` when it is a usable one newer than
    /// `self`.
    pub fn newer(&self, json: &str) -> Option<Catalog> {
        let c: Catalog = serde_json::from_str(json).ok()?;
        (c.version > self.version && !c.providers.is_empty()).then_some(c)
    }

    pub fn provider(&self, id: &str) -> Option<&ProviderInfo> {
        self.providers.iter().find(|p| p.id == id)
    }

    /// The services that make pictures (have at least one model).
    #[cfg(test)]
    pub fn picture_services(&self) -> impl Iterator<Item = &ProviderInfo> {
        self.providers.iter().filter(|p| !p.models.is_empty())
    }

    /// The services that can do `op` to a picture.
    pub fn services_for(&self, op: Op) -> Vec<&ProviderInfo> {
        self.providers
            .iter()
            .filter(|p| match op {
                Op::Generate | Op::Remix => {
                    p.models.iter().any(|m| m.ops.iter().any(|o| o == op.id()))
                }
                _ => p.op(op).is_some(),
            })
            .collect()
    }
}

impl ProviderInfo {
    pub fn model(&self, id: &str) -> Option<&ModelInfo> {
        self.models.iter().find(|m| m.id == id)
    }


    /// The models that can do `op` (Generate or Remix).
    pub fn models_for(&self, op: Op) -> Vec<&ModelInfo> {
        self.models
            .iter()
            .filter(|m| m.ops.iter().any(|o| o == op.id()))
            .collect()
    }

    pub fn op(&self, op: Op) -> Option<&OpInfo> {
        self.ops.iter().find(|o| o.op == op.id())
    }
}

impl OpInfo {
    /// The variant to use: `wanted` when there is one, else the first.
    pub fn pick(&self, wanted: &str) -> Option<&Variant> {
        self.variants
            .iter()
            .find(|v| v.id == wanted)
            .or_else(|| self.variants.first())
    }
}

impl ModelInfo {
    /// The size to use: `wanted` when the model makes it, else the
    /// largest it makes below it, else its first.
    pub fn fit_resolution(&self, wanted: &str) -> String {
        let order = ["1K", "2K", "4K"];
        let w = order.iter().position(|r| *r == wanted).unwrap_or(0);
        order[..=w]
            .iter()
            .rev()
            .find(|r| self.resolutions.iter().any(|x| x == *r))
            .map(|r| r.to_string())
            .or_else(|| self.resolutions.first().cloned())
            .unwrap_or_else(|| "1K".into())
    }

    /// The shape to use: `wanted` when the model makes it, else square,
    /// else its first.
    pub fn fit_aspect(&self, wanted: &str) -> String {
        if self.aspects.iter().any(|a| a == wanted) {
            return wanted.to_string();
        }
        if self.aspects.iter().any(|a| a == "1:1") {
            return "1:1".into();
        }
        self.aspects.first().cloned().unwrap_or_else(|| "1:1".into())
    }

    /// The price line for one picture at `res`.
    pub fn price_caption(&self, res: &str) -> Option<String> {
        use crate::i18n::trf;
        let line = if let Some(p) = self.price.get(res) {
            trf("aiimg.price_each", &[("price", &dollars(*p))])
        } else if let Some((lo, hi)) = self.price_range {
            trf(
                "aiimg.price_range",
                &[("low", &dollars(lo)), ("high", &dollars(hi))],
            )
        } else {
            return non_empty(text(&self.price_note));
        };
        Some(match non_empty(text(&self.price_note)) {
            Some(note) => format!("{line}. {note}"),
            None => line,
        })
    }
}

impl Variant {
    /// The price line for one run.
    pub fn price_caption(&self) -> Option<String> {
        let line = self
            .price
            .map(|p| crate::i18n::trf("studio.price_run", &[("price", &dollars(p))]));
        match (line, non_empty(text(&self.price_note))) {
            (Some(l), Some(n)) => Some(format!("{l}. {n}")),
            (Some(l), None) => Some(l),
            (None, n) => n,
        }
    }
}

fn non_empty(s: String) -> Option<String> {
    (!s.trim().is_empty()).then_some(s)
}

/// Dollars with the interface's decimal separator.
pub fn dollars(v: f64) -> String {
    let s = if v < 0.1 { format!("{v:.3}") } else { format!("{v:.2}") };
    let s = s.trim_end_matches('0').trim_end_matches('.').to_string();
    crate::ai_image::localize_decimal(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_built_in_catalog_reads_and_names_every_service() {
        let c: Catalog = serde_json::from_str(BUILT_IN).expect("catalog reads");
        assert!(c.version >= 1);
        for id in ["gemini", "openai", "recraft", "freepik", "stability"] {
            let p = c.provider(id).unwrap_or_else(|| panic!("{id} missing"));
            assert!(!p.name.is_empty() && p.key_url.starts_with("https://"), "{id}");
            for m in &p.models {
                assert!(!m.ops.is_empty() && !m.aspects.is_empty() && !m.resolutions.is_empty(), "{id}/{}", m.id);
                for o in &m.ops {
                    assert!(Op::from_id(o).is_some(), "{id}/{}: {o}", m.id);
                }
            }
            for o in &p.ops {
                assert!(Op::from_id(&o.op).is_some(), "{id}: {}", o.op);
                assert!(!o.variants.is_empty(), "{id}: {}", o.op);
            }
        }
        // Every picture service has a newest model.
        for p in c.picture_services() {
            assert!(p.models.iter().any(|m| m.newest), "{}", p.id);
        }
    }

    #[test]
    fn a_fetched_catalog_replaces_only_an_older_one() {
        let c = Catalog::built_in();
        let mut newer = c.clone();
        newer.version += 1;
        assert!(c.newer(&serde_json::to_string(&newer).expect("json")).is_some());
        assert!(c.newer(&serde_json::to_string(&c).expect("json")).is_none());
        assert!(c.newer("not json").is_none());
        let empty = Catalog { version: c.version + 5, ..Default::default() };
        assert!(c.newer(&serde_json::to_string(&empty).expect("json")).is_none());
    }

    #[test]
    fn sizes_and_shapes_fall_back_to_what_the_model_makes() {
        let m = ModelInfo {
            aspects: vec!["1:1".into(), "16:9".into()],
            resolutions: vec!["1K".into(), "2K".into()],
            ..Default::default()
        };
        assert_eq!(m.fit_resolution("4K"), "2K");
        assert_eq!(m.fit_resolution("1K"), "1K");
        assert_eq!(m.fit_aspect("16:9"), "16:9");
        assert_eq!(m.fit_aspect("21:9"), "1:1");
    }
}
