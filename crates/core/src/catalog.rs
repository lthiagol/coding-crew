//! The models a harness lists, and the effort levels each one takes.
//!
//! An agent's model is picked from its harness's own list, so the list is never
//! written out in hugo or raul. The table in [`crate::harness`] says where each
//! harness lists its models — a curated table, or a command read by its
//! [`Listing`]. A model's levels come from the same list: a model that lists
//! none takes no effort, and an agent on it has none.
//!
//! A command is bounded: a tool that hangs, is missing or fails is an error the
//! caller shows, never a window that waits on it.

use std::collections::BTreeMap;
use std::time::Duration;

use serde_json::Value;

use crate::bounded;
use crate::config::Agent;
use crate::harness::{CuratedModel, HARNESSES, Harness, Listing, Models};

/// How long a catalogue's tool may take before the read is given up.
///
/// A catalogue is a list of names: a tool that takes longer than this is not
/// answering, and the human is not left waiting on it.
pub const READ_BOUND: Duration = Duration::from_secs(5);

/// The effort levels, lowest first: the order a model's levels are listed in.
const LEVELS: [&str; 7] = ["none", "minimal", "low", "medium", "high", "xhigh", "max"];

/// One model a harness lists, and the levels it takes, lowest first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model {
    pub id: String,
    /// The levels the model takes. Empty: the model takes none.
    pub efforts: Vec<String>,
    /// The listing names the model with no level as well, so it runs with none.
    pub bare: bool,
}

/// The models a harness lists, in the order the list gives them.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Catalog {
    models: Vec<Model>,
}

impl Catalog {
    /// The catalogue a tool printed, one model per line, with no levels. A line
    /// is a model id, or an id and its label after ` - `. Blank lines are
    /// dropped, a model listed twice is one model, and a line that is not a
    /// model id is not a pick.
    pub fn from_lines(text: &str) -> Self {
        let mut catalog = Self::default();
        for line in text.lines() {
            if let Some(model) = model_of(line) {
                catalog.add(model, None);
            }
        }
        catalog
    }

    /// The catalogue the table holds for a harness.
    pub fn from_curated(models: &[CuratedModel]) -> Self {
        Self {
            models: models
                .iter()
                .map(|model| Model {
                    id: model.id.to_string(),
                    efforts: model
                        .efforts
                        .iter()
                        .map(|level| level.to_string())
                        .collect(),
                    bare: false,
                })
                .collect(),
        }
    }

    /// The catalogue a harness lists.
    pub fn read(harness: &Harness) -> Result<Self, String> {
        Self::read_within(harness, READ_BOUND)
    }

    /// The catalogue a harness lists, given up after `bound` when it is a
    /// command.
    pub fn read_within(harness: &Harness, bound: Duration) -> Result<Self, String> {
        match harness.models {
            Models::Curated(models) => Ok(Self::from_curated(models)),
            Models::Command { args, listing } => {
                if !harness.on_path() {
                    return Err(format!("{} is not on PATH", harness.binary));
                }
                Self::read_command(harness.binary, args, listing, bound)
            }
        }
    }

    /// The catalogue a program lists with `args`, read by `listing` and given
    /// up after `bound`.
    ///
    /// The tool is watched, not waited on: one that answers is read, and one
    /// still running at the bound is killed and reported.
    pub fn read_command(
        tool: &str,
        args: &[&str],
        listing: Listing,
        bound: Duration,
    ) -> Result<Self, String> {
        let command = std::iter::once(tool)
            .chain(args.iter().copied())
            .collect::<Vec<&str>>()
            .join(" ");
        let answer =
            bounded::run(tool, args, bound).map_err(|error| format!("{command}: {error}"))?;
        if !answer.status.success() {
            let why = String::from_utf8_lossy(&answer.stderr);
            let why = why.trim();
            return Err(if why.is_empty() {
                format!("{command} failed")
            } else {
                format!("{command} failed: {why}")
            });
        }
        let text =
            String::from_utf8(answer.stdout).map_err(|error| format!("{command}: {error}"))?;
        listing
            .parse(&text)
            .map_err(|why| format!("{command}: {why}"))
    }

    /// The models, in the order the catalogue lists them.
    pub fn models(&self) -> &[Model] {
        &self.models
    }

    /// The ids of the models, in the order the catalogue lists them.
    pub fn ids(&self) -> Vec<&str> {
        self.models.iter().map(|model| model.id.as_str()).collect()
    }

    /// Whether the catalogue knows a model string.
    ///
    /// The variant is part of the string, not of the catalogue: a pick of
    /// `opencode-go/glm-5.3` is the model `opencode-go/glm-5.3#high` too.
    pub fn knows(&self, model: &str) -> bool {
        self.index_of(model).is_some()
    }

    /// The position of a model in the catalogue, from its name.
    pub fn index_of(&self, model: &str) -> Option<usize> {
        let base = base_of(model);
        self.models
            .iter()
            .position(|known| base_of(&known.id) == base)
    }

    /// The levels a model takes. `None` when the catalogue does not list the
    /// model, so its levels are not known; `Some` of an empty list is a model
    /// that takes none.
    pub fn efforts_of(&self, model: &str) -> Option<&[String]> {
        self.index_of(model)
            .map(|index| self.models[index].efforts.as_slice())
    }

    /// Whether the catalogue lists the model at this level.
    pub fn takes(&self, model: &str, effort: &str) -> bool {
        self.efforts_of(model)
            .is_some_and(|levels| levels.iter().any(|level| level == effort))
    }

    /// Whether the model runs with no level: it lists none, or the listing
    /// names it bare as well as at its levels. A model the catalogue does not
    /// list is not known to take none.
    pub fn takes_none(&self, model: &str) -> bool {
        self.index_of(model)
            .is_some_and(|index| self.models[index].efforts.is_empty() || self.models[index].bare)
    }

    /// The entries an effort list offers for a model the catalogue does not
    /// list, so its levels are not known: every level the listed models take,
    /// lowest first, then [`NONE_ENTRY`], then [`TYPE_A_LEVEL`]. `none` is
    /// listed once, as the no-flag entry, even where a listed model names it.
    pub fn unlisted_choices(&self) -> Vec<String> {
        let mut levels: Vec<String> = Vec::new();
        for model in &self.models {
            for level in &model.efforts {
                if level != NONE_ENTRY && !levels.contains(level) {
                    levels.push(level.clone());
                }
            }
        }
        levels.sort_by_key(|level| rank(level));
        levels.push(NONE_ENTRY.to_string());
        levels.push(TYPE_A_LEVEL.to_string());
        levels
    }

    /// Add a model, or a level to one already listed. A model is kept once,
    /// and its levels once each.
    fn add(&mut self, id: &str, level: Option<&str>) {
        let index = match self.models.iter().position(|model| model.id == id) {
            Some(index) => index,
            None => {
                self.models.push(Model {
                    id: id.to_string(),
                    efforts: Vec::new(),
                    bare: false,
                });
                self.models.len() - 1
            }
        };
        if let Some(level) = level {
            let efforts = &mut self.models[index].efforts;
            if !efforts.iter().any(|known| known == level) {
                efforts.push(level.to_string());
            }
        }
    }

    /// Put each model's levels in the order they are named by [`LEVELS`]. A
    /// level the order does not name keeps its place after those it does.
    fn sort_levels(&mut self) {
        for model in &mut self.models {
            model.efforts.sort_by_key(|level| rank(level));
        }
    }
}

/// The last entry of a model list: it opens the inline input for an id the
/// catalogue does not list.
pub const TYPE_AN_ID: &str = "type an id…";

/// The last entry of an effort list for a model the catalogue does not list:
/// it opens the inline input for a level.
pub const TYPE_A_LEVEL: &str = "type a level…";

/// The entry that starts a model with no effort flag. On a model the catalogue
/// does not list it is the only way to say so; a listed model's `none` is its
/// own level, and stays `none`.
pub const NONE_ENTRY: &str = "none";

/// The effort a `none` entry on an unlisted model stores: an empty level, which
/// the harness starts without a flag.
pub const NO_EFFORT: &str = "";

/// The effort as a word: a stored [`NO_EFFORT`] reads as [`NONE_ENTRY`].
pub fn effort_word(effort: &str) -> &str {
    if effort == NO_EFFORT {
        NONE_ENTRY
    } else {
        effort
    }
}

/// The effort a picked entry stores. On a model the catalogue does not list,
/// `none` stores [`NO_EFFORT`]; otherwise the entry is the level itself.
pub fn stored_effort(entry: &str, listed: bool) -> String {
    if entry == NONE_ENTRY && !listed {
        NO_EFFORT.to_string()
    } else {
        entry.to_string()
    }
}

/// Whether a typed level has the shape of one: one word, with no `#`, since
/// the variant separator is the harness's, not the level's. Nothing else is
/// checked: the levels of an unlisted model are not known.
pub fn is_typed_level(text: &str) -> bool {
    let level = text.trim();
    !level.is_empty() && !level.chars().any(|c| c.is_whitespace() || c == '#')
}

/// The levels each harness's models take, as the reads so far have said.
///
/// A harness the table holds with its models in it has them from the start;
/// a command harness's models are known once its listing has been read. A
/// model the levels are not known for is one its agent cannot be checked
/// against, so its effort is a gap.
#[derive(Debug, Default)]
pub struct Levels {
    catalogs: BTreeMap<String, Catalog>,
}

impl Levels {
    /// The levels the table holds, for the harnesses that hold their models.
    pub fn new() -> Self {
        let mut levels = Self::default();
        for harness in HARNESSES {
            if let Models::Curated(_) = harness.models
                && let Ok(catalog) = Catalog::read(&harness)
            {
                levels.insert(harness.name, catalog);
            }
        }
        levels
    }

    /// Keep the catalogue a harness's listing has answered with.
    pub fn insert(&mut self, harness: &str, catalog: Catalog) {
        self.catalogs.insert(harness.to_string(), catalog);
    }

    /// The catalogue a harness has answered with, when it has.
    pub fn catalog(&self, harness: &str) -> Option<&Catalog> {
        self.catalogs.get(harness)
    }

    /// The levels an agent's model takes, when they are known.
    pub fn of(&self, agent: &Agent) -> Option<&[String]> {
        self.catalog(&agent.harness)?.efforts_of(&agent.model)
    }

    /// Whether an agent's model is known to take no level at all, so it needs
    /// no effort.
    pub fn takes_none(&self, agent: &Agent) -> bool {
        self.catalog(&agent.harness)
            .is_some_and(|catalog| catalog.takes_none(&agent.model))
    }
}

/// Switch an agent to another harness.
///
/// The model stays only when the new harness lists it, and is cleared
/// otherwise, so the agent says it needs a model rather than running one the
/// tool does not know. The effort stays only when the model takes it, and
/// otherwise is cleared: an effort is a level of a model, so with no model
/// there is no level to keep.
pub fn switch_harness(agent: &mut Agent, to: &Harness, catalog: &Catalog) {
    agent.harness = to.name.to_string();
    if !catalog.knows(&agent.model) {
        agent.model.clear();
    }
    let kept = agent
        .effort
        .as_deref()
        .is_some_and(|effort| catalog.takes(&agent.model, effort));
    if !kept {
        agent.effort = None;
    }
}

/// Whether a model pick keeps the agent's effort: the new model takes it. `levels`
/// are the new model's, and `None` when they are not known, which clears it.
pub fn keeps_effort(effort: Option<&str>, levels: Option<&[String]>) -> bool {
    effort.is_some_and(|effort| levels.is_some_and(|levels| levels.iter().any(|l| l == effort)))
}

impl Listing {
    /// The catalogue a listing printed.
    pub fn parse(self, text: &str) -> Result<Catalog, String> {
        match self {
            Listing::Lines => Ok(Catalog::from_lines(text)),
            Listing::Cursor => Ok(from_cursor(text)),
            Listing::Opencode => from_opencode(text),
        }
    }
}

/// The catalogue `cursor-agent --list-models` printed.
///
/// A parameterised model is listed once per level, as its id with the level
/// ending it: `claude-opus-5-5-high` is `claude-opus-5-5` at `high`, and
/// `claude-opus-5-5` itself is not listed. A base the listing also names bare
/// (`gpt-5.3-codex`) runs with no level as well, so it is marked `bare`.
/// `-fast` variants are the same models at a faster speed, not a level, so they
/// are skipped. `extra-high` is not split: `gpt-5.5-extra-high` stays whole, as
/// the id the tool knows it by.
fn from_cursor(text: &str) -> Catalog {
    let mut catalog = Catalog::default();
    for line in text.lines() {
        let Some(id) = model_of(line) else {
            continue;
        };
        if id.ends_with("-fast") {
            continue;
        }
        match split_level(id) {
            Some((base, level)) => catalog.add(base, Some(level)),
            None => {
                catalog.add(id, None);
                if let Some(model) = catalog.models.iter_mut().find(|model| model.id == id) {
                    model.bare = true;
                }
            }
        }
    }
    catalog.sort_levels();
    catalog
}

/// Split a cursor id into its model and its level, when it ends in one.
fn split_level(id: &str) -> Option<(&str, &str)> {
    LEVELS.iter().copied().find_map(|level| {
        let base = id.strip_suffix(level)?.strip_suffix('-')?;
        (!base.is_empty() && !base.ends_with("-extra")).then_some((base, level))
    })
}

/// The catalogue `opencode api --standalone model.list` printed: JSON with a
/// `data` array of models. Each model is `providerID/id`, and its `variants`
/// are its levels. A model the server has disabled is not listed.
fn from_opencode(text: &str) -> Result<Catalog, String> {
    let value: Value = serde_json::from_str(text).map_err(|error| format!("not JSON: {error}"))?;
    let data = value
        .get("data")
        .and_then(Value::as_array)
        .ok_or_else(|| "no model list in the answer".to_string())?;
    let mut catalog = Catalog::default();
    for item in data {
        if item.get("enabled").and_then(Value::as_bool) == Some(false) {
            continue;
        }
        let Some(id) = item.get("id").and_then(Value::as_str) else {
            continue;
        };
        let id = match item.get("providerID").and_then(Value::as_str) {
            Some(provider) => format!("{provider}/{id}"),
            None => id.to_string(),
        };
        catalog.add(&id, None);
        for variant in item
            .get("variants")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(level) = variant.get("id").and_then(Value::as_str) {
                catalog.add(&id, Some(level));
            }
        }
    }
    catalog.sort_levels();
    Ok(catalog)
}

/// A level's place in [`LEVELS`]; a level the order does not name comes last.
fn rank(level: &str) -> usize {
    LEVELS
        .iter()
        .position(|known| *known == level)
        .unwrap_or(usize::MAX)
}

/// The model id a line of a harness's list names, when it names one.
///
/// A harness prints its models one to a line, with a label after ` - ` on some
/// (`auto - Auto (default)`). A banner or a header can sit among them, and only
/// a model id is pickable.
fn model_of(line: &str) -> Option<&str> {
    let line = line.trim();
    let id = line.split_once(" - ").map_or(line, |(id, _)| id.trim_end());
    is_model_id(id).then_some(id)
}

/// Whether a word is a model id: no whitespace, a letter or a digit first, and
/// no empty part between slashes. A model may route through more providers
/// (`openrouter/anthropic/claude-sonnet-4`), and may carry a tag
/// (`ollama/llama3:8b`), so it may hold slashes and colons.
pub fn is_model_id(id: &str) -> bool {
    !id.is_empty()
        && !id.chars().any(char::is_whitespace)
        && id.starts_with(|character: char| character.is_ascii_alphanumeric())
        && id.split('/').all(|part| !part.is_empty())
}

/// A model's name without its variant: `provider/model#high` is the model
/// `provider/model`.
pub fn base_of(model: &str) -> &str {
    model.split_once('#').map_or(model, |(base, _)| base)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness;
    use std::time::Instant;

    const CURSOR_LIST: &str = include_str!("../tests/listings/cursor-agent-list-models.txt");
    const OPENCODE_LIST: &str = include_str!("../tests/listings/opencode-model-list.json");

    /// A scratch folder holding an executable tool that answers with the
    /// script's own words. The caller removes the folder.
    fn scratch_tool(name: &str, script: &str) -> std::path::PathBuf {
        #[cfg(unix)]
        use std::os::unix::fs::PermissionsExt;

        let dir = std::env::temp_dir().join(format!("core-catalog-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a scratch folder");
        let path = dir.join("a-tool");
        std::fs::write(&path, script).expect("the tool");
        #[cfg(unix)]
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
            .expect("an executable tool");
        path
    }

    fn levels(catalog: &Catalog, model: &str) -> Vec<String> {
        catalog
            .efforts_of(model)
            .unwrap_or_else(|| panic!("{model} is not listed"))
            .to_vec()
    }

    #[test]
    fn a_curated_harness_lists_its_models_with_their_levels() {
        let claude = harness::find("claude").expect("claude");
        let catalog = Catalog::read(claude).expect("the curated list");
        assert!(catalog.knows("claude-haiku-5-5"));
        assert!(catalog.knows("opus"));
        assert!(!catalog.knows("opencode-go/glm-5.3"));
        assert_eq!(
            levels(&catalog, "opus"),
            ["low", "medium", "high", "xhigh", "max"]
        );
        assert_eq!(
            levels(&catalog, "claude-haiku-5-5"),
            ["low", "medium", "high"]
        );
    }

    #[test]
    fn a_label_after_the_id_is_not_part_of_the_model() {
        // cursor-agent prints `id - Label`; the id is the model.
        let catalog =
            Catalog::from_lines("Available models\n\nauto - Auto (default)\ngpt-5 - GPT-5\n");
        assert_eq!(catalog.ids(), ["auto", "gpt-5"]);
    }

    #[test]
    fn the_catalogue_is_the_lines_the_tool_printed() {
        let catalog = Catalog::from_lines(
            "opencode-go/deepseek-v4.1-flash\n\n  opencode-go/glm-5.3  \nopencode-go/kimi-k3\n",
        );
        assert_eq!(
            catalog.ids(),
            [
                "opencode-go/deepseek-v4.1-flash",
                "opencode-go/glm-5.3",
                "opencode-go/kimi-k3"
            ]
        );
        assert!(
            catalog
                .efforts_of("opencode-go/glm-5.3")
                .expect("listed")
                .is_empty()
        );
    }

    #[test]
    fn a_model_listed_twice_is_one_model() {
        let catalog = Catalog::from_lines("a/b\na/b\nc/d\n");
        assert_eq!(catalog.ids(), ["a/b", "c/d"]);
    }

    #[test]
    fn the_variant_is_part_of_a_model_the_catalogue_knows() {
        let catalog = Catalog::from_lines("opencode-go/glm-5.3\nopencode-go/kimi-k3\n");
        assert!(catalog.knows("opencode-go/glm-5.3"));
        assert!(catalog.knows("opencode-go/glm-5.3#high"));
        assert!(catalog.knows("opencode-go/kimi-k3#low"));
        // A model the catalogue does not list is not a model.
        assert!(!catalog.knows("opencode-go/nothing"));
        assert!(!catalog.knows("opencode-go/nothing#high"));
        // The provider is part of the model's name: a bare name is not one.
        assert!(!catalog.knows("glm-5.3"));
        // A catalogue that printed the variant is the same model.
        let with_variant = Catalog::from_lines("opencode-go/glm-5.3#low\n");
        assert!(with_variant.knows("opencode-go/glm-5.3"));
        assert!(with_variant.knows("opencode-go/glm-5.3#high"));
        assert_eq!(with_variant.index_of("opencode-go/glm-5.3#high"), Some(0));
    }

    #[test]
    fn a_line_that_is_not_a_model_id_is_not_a_pick() {
        // The tool's banner and separators are not models: a pick must be a
        // model id, with a provider and a model name.
        let catalog = Catalog::from_lines(
            "Available models:\n---\nopencode-go/glm-5.3\nTotal: 1 model\n/glm\nopencode-go/\n",
        );
        assert_eq!(catalog.ids(), ["opencode-go/glm-5.3"]);
        assert!(!catalog.knows("Available models:"));
    }

    #[test]
    fn a_model_with_a_tag_is_a_pick() {
        // A tag or a suffix after a colon is part of the model's id, so the
        // catalogue keeps it, and a variant still resolves to it.
        let catalog = Catalog::from_lines("ollama/llama3:8b\nopenrouter/x/y:free\n");
        assert_eq!(catalog.ids(), ["ollama/llama3:8b", "openrouter/x/y:free"]);
        assert!(catalog.knows("ollama/llama3:8b#high"));
        assert!(catalog.knows("openrouter/x/y:free"));
    }

    #[test]
    fn a_model_routed_through_more_providers_is_a_pick() {
        // `opencode models` prints routed ids such as
        // `openrouter/anthropic/claude-sonnet-4`: the rest of the id is the
        // model, slashes and all.
        let catalog = Catalog::from_lines("openrouter/anthropic/claude-sonnet-4\n");
        assert_eq!(catalog.ids(), ["openrouter/anthropic/claude-sonnet-4"]);
        assert!(catalog.knows("openrouter/anthropic/claude-sonnet-4#high"));
    }

    #[test]
    fn an_empty_list_is_an_empty_catalogue() {
        let catalog = Catalog::from_lines("\n  \n");
        assert!(catalog.models().is_empty());
        assert!(!catalog.knows("a/b"));
    }

    #[test]
    fn a_model_takes_a_level_only_when_its_list_names_it() {
        let catalog = Catalog::from_curated(&[CuratedModel {
            id: "m",
            efforts: &["low", "high"],
        }]);
        assert!(catalog.takes("m", "high"));
        assert!(!catalog.takes("m", "xhigh"));
        assert!(!catalog.takes("nothing", "high"));
    }

    #[test]
    fn an_unlisted_model_is_offered_the_harness_levels_then_none_then_a_typed_level() {
        // The levels every listed model takes, lowest first; `none` once, as
        // the no-flag entry; the typed level last.
        let catalog = from_opencode(OPENCODE_LIST).expect("the captured list");
        assert_eq!(
            catalog.unlisted_choices(),
            [
                "minimal",
                "low",
                "medium",
                "high",
                "xhigh",
                "max",
                NONE_ENTRY,
                TYPE_A_LEVEL
            ]
        );
        let claude = Catalog::read(harness::find("claude").expect("claude")).expect("the table");
        assert_eq!(
            claude.unlisted_choices(),
            [
                "low",
                "medium",
                "high",
                "xhigh",
                "max",
                NONE_ENTRY,
                TYPE_A_LEVEL
            ]
        );
        // A catalogue with no levels offers only the two entries.
        assert_eq!(
            Catalog::from_lines("p/m\n").unlisted_choices(),
            [NONE_ENTRY, TYPE_A_LEVEL]
        );
    }

    #[test]
    fn none_on_an_unlisted_model_stores_no_effort_and_on_a_listed_one_its_level() {
        assert_eq!(stored_effort(NONE_ENTRY, false), NO_EFFORT);
        assert_eq!(stored_effort(NONE_ENTRY, true), NONE_ENTRY);
        assert_eq!(stored_effort("high", false), "high");
        assert_eq!(effort_word(NO_EFFORT), NONE_ENTRY);
        assert_eq!(effort_word("high"), "high");
    }

    #[test]
    fn a_typed_level_is_one_word_with_no_variant_separator() {
        assert!(is_typed_level("high"));
        assert!(is_typed_level("  ultra-2  "));
        assert!(!is_typed_level(""));
        assert!(!is_typed_level("  "));
        assert!(!is_typed_level("very high"));
        assert!(!is_typed_level("high#low"));
    }

    #[test]
    fn cursor_lists_each_parameterised_model_once_with_its_levels() {
        let catalog = from_cursor(CURSOR_LIST);
        assert_eq!(
            levels(&catalog, "claude-opus-5-5"),
            ["low", "medium", "high", "xhigh", "max"]
        );
        assert_eq!(levels(&catalog, "gpt-5.3-codex"), ["low", "high", "xhigh"]);
        assert_eq!(
            levels(&catalog, "deepseek-v41-flash-1017"),
            ["none", "low", "medium", "high", "max"]
        );
        // The id a level ends is not a model of its own.
        assert!(!catalog.knows("claude-opus-5-5-high"));
        assert!(!catalog.knows("claude-opus-5-5-high-fast"));
    }

    #[test]
    fn cursor_keeps_a_model_with_no_level_and_no_split_extra_high() {
        let catalog = from_cursor(CURSOR_LIST);
        assert!(catalog.knows("auto"));
        assert!(levels(&catalog, "auto").is_empty());
        assert!(levels(&catalog, "composer-2.5").is_empty());
        // `gpt-5.5-extra-high` is the id the tool knows, so it stays whole.
        assert!(catalog.knows("gpt-5.5-extra-high"));
        assert!(!catalog.knows("gpt-5.5-extra"));
    }

    #[test]
    fn cursor_ignores_its_banner_and_tip() {
        let catalog = from_cursor(CURSOR_LIST);
        assert!(!catalog.ids().iter().any(|id| id.contains(' ')));
        assert!(!catalog.knows("Available"));
        assert!(!catalog.knows("Tip:"));
    }

    #[test]
    fn cursor_levels_are_named_in_their_order() {
        let catalog = from_cursor("muse-x-high\nmuse-x-none\nmuse-x-max\nmuse-x-low\n");
        assert_eq!(levels(&catalog, "muse-x"), ["none", "low", "high", "max"]);
    }

    #[test]
    fn cursor_names_a_base_bare_as_well_as_at_its_levels() {
        // `gpt-5.3-codex` is listed bare and at its levels: it runs with none.
        let catalog = from_cursor("gpt-5.3-codex-low\ngpt-5.3-codex\ngpt-5.3-codex-high\n");
        assert_eq!(levels(&catalog, "gpt-5.3-codex"), ["low", "high"]);
        assert!(catalog.takes_none("gpt-5.3-codex"));
        // A base listed only at its levels runs with one of them.
        let only_levels = from_cursor("muse-x-low\nmuse-x-high\n");
        assert!(!only_levels.takes_none("muse-x"));
        // A model with no level at all takes none, and an unlisted one is not
        // known to.
        assert!(from_cursor("auto\n").takes_none("auto"));
        assert!(!catalog.takes_none("nothing"));
    }

    #[test]
    fn cursor_lists_gpt_5_3_codex_bare_and_claude_opus_only_at_levels() {
        let catalog = from_cursor(CURSOR_LIST);
        assert!(catalog.takes_none("gpt-5.3-codex"));
        assert!(!catalog.takes_none("claude-opus-5-5"));
    }

    #[test]
    fn opencode_lists_each_model_with_its_variants_as_levels() {
        let catalog = from_opencode(OPENCODE_LIST).expect("the captured list");
        assert_eq!(catalog.models().len(), 10);
        assert!(catalog.knows("opencode/beam-free"));
        assert_eq!(
            levels(&catalog, "opencode/beam-free"),
            ["low", "medium", "high", "xhigh", "max"]
        );
        assert_eq!(
            levels(&catalog, "opencode/deepseek-v4.1-flash-free"),
            ["low", "high", "max"]
        );
        assert_eq!(
            levels(&catalog, "opencode/hy4-flash-free"),
            ["none", "high"]
        );
        // big-pickle has no variants: it is listed, and takes no level.
        assert!(levels(&catalog, "opencode/big-pickle").is_empty());
    }

    #[test]
    fn opencode_lists_the_same_models_as_its_plain_list() {
        // `opencode models` prints the same set, in its own order.
        let plain = Catalog::from_lines(include_str!("../tests/listings/opencode-models.txt"));
        let catalog = from_opencode(OPENCODE_LIST).expect("the captured list");
        let mut listed = catalog.ids();
        let mut plain_ids = plain.ids();
        listed.sort_unstable();
        plain_ids.sort_unstable();
        assert_eq!(listed, plain_ids);
    }

    #[test]
    fn opencode_skips_a_model_the_server_has_disabled() {
        let text = r#"{"data":[{"id":"a","providerID":"p","enabled":false},{"id":"b","providerID":"p","enabled":true}]}"#;
        let catalog = from_opencode(text).expect("a list");
        assert_eq!(catalog.ids(), ["p/b"]);
    }

    #[test]
    fn an_answer_that_is_not_a_model_list_is_an_error() {
        assert!(
            from_opencode("<!doctype html>")
                .unwrap_err()
                .contains("not JSON")
        );
        assert!(from_opencode("{}").unwrap_err().contains("no model list"));
    }

    #[test]
    fn a_tool_that_is_not_there_cannot_be_read() {
        let error = Catalog::read_command(
            "coding-crew-no-such-tool-for-catalog",
            &[],
            Listing::Lines,
            READ_BOUND,
        )
        .expect_err("there is no such tool");
        assert!(
            error.contains("coding-crew-no-such-tool-for-catalog"),
            "{error}"
        );
    }

    #[test]
    fn a_tool_that_answers_is_read_without_waiting_for_the_bound() {
        let tool = scratch_tool("answers", "#!/bin/sh\necho p/m0\necho p/m1\n");
        let started = Instant::now();
        let catalog = Catalog::read_command(
            &tool.to_string_lossy(),
            &[],
            Listing::Lines,
            Duration::from_secs(30),
        )
        .expect("the tool answers");
        assert_eq!(catalog.ids(), ["p/m0", "p/m1"]);
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "a read that answered was held to its bound"
        );
        let _ = std::fs::remove_dir_all(tool.parent().expect("the scratch folder"));
    }

    #[test]
    fn a_tool_that_answers_in_the_wrong_format_is_an_error_that_names_the_tool() {
        let tool = scratch_tool("wrong", "#!/bin/sh\necho '<html>'\n");
        let error = Catalog::read_command(
            &tool.to_string_lossy(),
            &[],
            Listing::Opencode,
            Duration::from_secs(5),
        )
        .expect_err("an HTML page is not a model list");
        assert!(error.contains("a-tool"), "{error}");
        assert!(error.contains("not JSON"), "{error}");
        let _ = std::fs::remove_dir_all(tool.parent().expect("the scratch folder"));
    }

    #[test]
    fn a_tool_that_hangs_is_given_up_at_the_bound() {
        // A bound on the wait, not on the tool: a tool that never answers is
        // an error the caller can show, returned inside the bound.
        let tool = scratch_tool("hangs", "#!/bin/sh\nexec sleep 30\n");
        let started = Instant::now();
        let error = Catalog::read_command(
            &tool.to_string_lossy(),
            &[],
            Listing::Lines,
            Duration::from_millis(100),
        )
        .expect_err("a hanging tool cannot be read");
        assert!(error.contains("timed out"), "{error}");
        assert!(error.contains("a-tool"), "{error}");
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "the read outlived its bound: {:?}",
            started.elapsed()
        );
        let _ = std::fs::remove_dir_all(tool.parent().expect("the scratch folder"));
    }

    #[test]
    fn a_tool_that_fails_names_why() {
        let tool = scratch_tool(
            "fails",
            "#!/bin/sh\necho 'the harness is not configured' >&2\nexit 3\n",
        );
        let error = Catalog::read_command(
            &tool.to_string_lossy(),
            &[],
            Listing::Lines,
            Duration::from_secs(5),
        )
        .expect_err("a failing tool cannot be read");
        assert!(error.contains("failed"), "{error}");
        assert!(error.contains("the harness is not configured"), "{error}");
        let _ = std::fs::remove_dir_all(tool.parent().expect("the scratch folder"));
    }

    #[test]
    fn a_tool_that_prints_more_than_a_pipe_holds_is_read_whole() {
        // The output is drained while the tool runs: a tool that fills the
        // pipe would otherwise block until the bound killed it.
        let tool = scratch_tool(
            "long",
            "#!/bin/sh\nawk 'BEGIN { for (i = 0; i < 12000; i++) print \"p/m\" i }'\n",
        );
        let catalog = Catalog::read_command(
            &tool.to_string_lossy(),
            &[],
            Listing::Lines,
            Duration::from_secs(30),
        )
        .expect("the long catalogue");
        assert_eq!(catalog.models().len(), 12000);
        assert_eq!(catalog.ids().last().copied(), Some("p/m11999"));
        let _ = std::fs::remove_dir_all(tool.parent().expect("the scratch folder"));
    }
}
