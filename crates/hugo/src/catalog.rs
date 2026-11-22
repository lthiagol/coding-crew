//! The models the catalog knows.
//!
//! The orchestrator's model is picked in the sidebar, so the list of models
//! is its harness's own: the table in `coding_crew_core::harness` says where
//! each harness lists its models — a curated list, or a command such as
//! `opencode models`, one model per line. The list is never written out here,
//! and a pick is checked against the same list it came from, so `config.toml`
//! cannot hold a model that does not exist.
//!
//! The effort is not part of a model: the catalogue lists the model, and the
//! agent's effort is a setting of its own.
//!
//! A command is bounded: a tool that hangs, is missing or fails is an error
//! the caller shows, never a window that waits on it.

use std::time::Duration;

use coding_crew_core::harness::{Harness, Models};

use crate::bounded;

/// How long the catalogue's tool may take before the read is given up.
///
/// A catalogue is a list of names: a tool that takes longer than this is not
/// answering, and the human is not left waiting on it.
pub const READ_BOUND: Duration = Duration::from_secs(5);

/// The models a harness's tool lists.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Catalog {
    models: Vec<String>,
}

impl Catalog {
    /// The catalogue a tool printed: one model per line, in the order it
    /// printed them. A line is a model id, or an id and its label after
    /// ` - `. Blank lines are dropped, a model listed twice is one model, and
    /// a line that is not a model id is not a pick.
    pub fn from_lines(text: &str) -> Self {
        let mut models: Vec<String> = Vec::new();
        for line in text.lines() {
            if let Some(model) = model_of(line)
                && !models.iter().any(|known| known == model)
            {
                models.push(model.to_string());
            }
        }
        Self { models }
    }

    /// The catalogue a harness's table holds.
    pub fn from_models(models: &[&str]) -> Self {
        Self {
            models: models.iter().map(|model| model.to_string()).collect(),
        }
    }

    /// The catalogue a harness lists its models in.
    pub fn read(harness: &Harness) -> Result<Self, String> {
        Self::read_within(harness, READ_BOUND)
    }

    /// The catalogue a harness lists, given up after `bound` when it is a
    /// command.
    pub fn read_within(harness: &Harness, bound: Duration) -> Result<Self, String> {
        match harness.models {
            Models::Curated(models) => Ok(Self::from_models(models)),
            Models::Command(args) => {
                if !harness.on_path() {
                    return Err(format!("{} is not on PATH", harness.binary));
                }
                Self::read_command(harness.binary, args, bound)
            }
        }
    }

    /// The catalogue a program lists with `args`, given up after `bound`.
    ///
    /// The tool is watched, not waited on: one that answers is read, and one
    /// still running at the bound is killed and reported. The bound is a
    /// timeout on the monotonic clock, not the frame's instant, which only
    /// the loop reads.
    pub fn read_command(tool: &str, args: &[&str], bound: Duration) -> Result<Self, String> {
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
        Ok(Self::from_lines(&text))
    }

    /// The models, in the order the catalogue lists them.
    pub fn models(&self) -> &[String] {
        &self.models
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
        self.models.iter().position(|known| base_of(known) == base)
    }
}

/// The model id a line of a harness's list names, when it names one.
///
/// A harness prints its models one to a line, with a label after ` - ` on
/// some (`auto - Auto (default)`). A banner or a header can sit among them,
/// and only a model id is pickable.
fn model_of(line: &str) -> Option<&str> {
    let line = line.trim();
    let id = line.split_once(" - ").map_or(line, |(id, _)| id.trim_end());
    is_model_id(id).then_some(id)
}

/// Whether a word is a model id: no whitespace, a letter or a digit first, and
/// no empty part between slashes. A model may route through more providers
/// (`openrouter/anthropic/claude-sonnet-4`), and may carry a tag
/// (`ollama/llama3:8b`), so it may hold slashes and colons.
fn is_model_id(id: &str) -> bool {
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

/// A scratch folder holding an executable tool that answers with the script's
/// own words. The caller removes the folder.
#[cfg(test)]
pub(crate) fn scratch_tool(name: &str, script: &str) -> std::path::PathBuf {
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    let dir = std::env::temp_dir().join(format!("hugo-catalog-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch folder");
    let path = dir.join("a-tool");
    std::fs::write(&path, script).expect("the tool");
    #[cfg(unix)]
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
        .expect("an executable tool");
    path
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn a_curated_harness_lists_the_models_its_table_holds() {
        let claude = coding_crew_core::harness::find("claude").expect("claude");
        let catalog = Catalog::read(claude).expect("the curated list");
        assert!(catalog.knows("claude-haiku-5-5"));
        assert!(catalog.knows("opus"));
        assert!(!catalog.knows("opencode-go/glm-5.3"));
    }

    #[test]
    fn a_label_after_the_id_is_not_part_of_the_model() {
        // cursor-agent prints `id - Label`; the id is the model.
        let catalog =
            Catalog::from_lines("Available models\n\nauto - Auto (default)\ngpt-5 - GPT-5\n");
        assert_eq!(catalog.models(), ["auto", "gpt-5"]);
    }

    #[test]
    fn the_catalogue_is_the_lines_the_tool_printed() {
        let catalog = Catalog::from_lines(
            "opencode-go/deepseek-v4.1-flash\n\n  opencode-go/glm-5.3  \nopencode-go/kimi-k3\n",
        );
        assert_eq!(
            catalog.models(),
            [
                "opencode-go/deepseek-v4.1-flash",
                "opencode-go/glm-5.3",
                "opencode-go/kimi-k3"
            ]
        );
    }

    #[test]
    fn a_model_listed_twice_is_one_model() {
        let catalog = Catalog::from_lines("a/b\na/b\nc/d\n");
        assert_eq!(catalog.models(), ["a/b", "c/d"]);
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
        assert_eq!(catalog.models(), ["opencode-go/glm-5.3"]);
        assert!(!catalog.knows("Available models:"));
    }

    #[test]
    fn a_model_with_a_tag_is_a_pick() {
        // A tag or a suffix after a colon is part of the model's id, so the
        // catalogue keeps it, and a variant still resolves to it.
        let catalog = Catalog::from_lines("ollama/llama3:8b\nopenrouter/x/y:free\n");
        assert_eq!(
            catalog.models(),
            ["ollama/llama3:8b", "openrouter/x/y:free"]
        );
        assert!(catalog.knows("ollama/llama3:8b#high"));
        assert!(catalog.knows("openrouter/x/y:free"));
    }

    #[test]
    fn a_model_routed_through_more_providers_is_a_pick() {
        // `opencode models` prints routed ids such as
        // `openrouter/anthropic/claude-sonnet-4`: the rest of the id is the
        // model, slashes and all.
        let catalog = Catalog::from_lines("openrouter/anthropic/claude-sonnet-4\n");
        assert_eq!(catalog.models(), ["openrouter/anthropic/claude-sonnet-4"]);
        assert!(catalog.knows("openrouter/anthropic/claude-sonnet-4#high"));
    }

    #[test]
    fn an_empty_list_is_an_empty_catalogue() {
        let catalog = Catalog::from_lines("\n  \n");
        assert!(catalog.models().is_empty());
        assert!(!catalog.knows("a/b"));
    }

    #[test]
    fn a_tool_that_is_not_there_cannot_be_read() {
        let error = Catalog::read_command("coding-crew-no-such-tool-for-hugo", &[], READ_BOUND)
            .expect_err("there is no such tool");
        assert!(
            error.contains("coding-crew-no-such-tool-for-hugo"),
            "{error}"
        );
    }

    #[test]
    fn a_tool_that_answers_is_read_without_waiting_for_the_bound() {
        let tool = scratch_tool("answers", "#!/bin/sh\necho p/m0\necho p/m1\n");
        let started = Instant::now();
        let catalog = Catalog::read_command(&tool.to_string_lossy(), &[], Duration::from_secs(30))
            .expect("the tool answers");
        assert_eq!(catalog.models(), ["p/m0", "p/m1"]);
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "a read that answered was held to its bound"
        );
        let _ = std::fs::remove_dir_all(tool.parent().expect("the scratch folder"));
    }

    #[test]
    fn a_tool_that_hangs_is_given_up_at_the_bound() {
        // A bound on the wait, not on the tool: a tool that never answers is
        // an error the caller can show, returned inside the bound.
        let tool = scratch_tool("hangs", "#!/bin/sh\nexec sleep 30\n");
        let started = Instant::now();
        let error = Catalog::read_command(&tool.to_string_lossy(), &[], Duration::from_millis(100))
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
    fn a_tool_whose_child_holds_its_output_is_given_up_at_the_bound() {
        // The tool answers and exits, but a child it left behind keeps the
        // output pipes open: the read is still bounded.
        let tool = scratch_tool("orphan", "#!/bin/sh\n(sleep 30) &\necho p/m\n");
        let started = Instant::now();
        let error = Catalog::read_command(&tool.to_string_lossy(), &[], Duration::from_millis(300))
            .expect_err("a read held by a child's pipes cannot be read");
        assert!(error.contains("timed out"), "{error}");
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
        let error = Catalog::read_command(&tool.to_string_lossy(), &[], Duration::from_secs(5))
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
        let catalog = Catalog::read_command(&tool.to_string_lossy(), &[], Duration::from_secs(30))
            .expect("the long catalogue");
        assert_eq!(catalog.models().len(), 12000);
        assert_eq!(
            catalog.models().last().map(String::as_str),
            Some("p/m11999")
        );
        let _ = std::fs::remove_dir_all(tool.parent().expect("the scratch folder"));
    }
}
