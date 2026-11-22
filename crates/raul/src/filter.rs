//! The Tasks tab's filter: one line of text, read as words and `key:value`
//! tokens.
//!
//! A plain word matches the id, the title or a tag, as a substring, ignoring
//! case; every word must match. A token matches one field of a task. Different
//! keys combine with AND, the same key repeated with OR. A token with an
//! unknown key or value matches nothing, and says so: the footer names it.

use std::fmt;

use coding_crew_core::task::{Effort, Priority, TaskKind};
use coding_crew_core::{Status, Task};

use crate::data::STATUSES;
use crate::tasks::{EFFORTS, PRIORITIES};

/// The kinds a task may be: the values `kind:` takes.
const KINDS: [TaskKind; 3] = [TaskKind::Task, TaskKind::Bug, TaskKind::Feature];

/// The text of a filter, read into its words and its tokens.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Filter {
    /// The plain words, in lower case: each must match a task.
    pub words: Vec<String>,
    /// The `key:value` tokens, in the order they were typed.
    pub tokens: Vec<Token>,
}

/// A `key:value` token, read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    /// The status, by name or prefix: every status the value names.
    Status(Vec<Status>),
    /// The priority.
    Priority(Priority),
    /// The effort.
    Effort(Effort),
    /// The task kind.
    Kind(TaskKind),
    /// A tag, exactly.
    Tag(String),
    /// Whether a claim is held.
    Claimed(bool),
    /// The subtasks of the task with this id.
    Parent(String),
    /// A token that is not one the filter knows: it matches nothing.
    Unknown(Error),
}

/// Why a token is not read: the key is unknown, or the value is not one the
/// key takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The key names no field.
    Key(String),
    /// The key takes no such value.
    Value { key: &'static str, value: String },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Key(key) => write!(f, "unknown key: {key}"),
            Error::Value { key, value } => write!(f, "unknown {key}: {value}"),
        }
    }
}

impl Filter {
    /// The filter the text makes. Text with no words and no tokens matches
    /// every task.
    pub fn parse(text: &str) -> Filter {
        let mut filter = Filter::default();
        for word in text.split_whitespace() {
            match word.split_once(':') {
                Some((key, value)) if !key.is_empty() => filter.tokens.push(token(key, value)),
                _ => filter.words.push(word.to_lowercase()),
            }
        }
        filter
    }

    /// Whether the filter has no words and no tokens: it is not in force.
    pub fn is_empty(&self) -> bool {
        self.words.is_empty() && self.tokens.is_empty()
    }

    /// The first token that is not read, as the error that names it.
    pub fn unknown(&self) -> Option<&Error> {
        self.tokens.iter().find_map(|token| match token {
            Token::Unknown(error) => Some(error),
            _ => None,
        })
    }

    /// Whether the task passes the filter.
    pub fn matches(&self, task: &Task) -> bool {
        self.words.iter().all(|word| word_matches(task, word))
            && self.tokens.iter().all(|token| {
                // The tokens of one key are OR'd: this token passes when
                // any token of its key does.
                self.tokens
                    .iter()
                    .filter(|other| other.key() == token.key())
                    .any(|other| other.matches(task))
            })
    }
}

impl Token {
    /// The key the token names. An unknown token has none of its own, so it
    /// shares no key with a token that could pass.
    fn key(&self) -> &'static str {
        match self {
            Token::Status(_) => "status",
            Token::Priority(_) => "priority",
            Token::Effort(_) => "effort",
            Token::Kind(_) => "kind",
            Token::Tag(_) => "tag",
            Token::Claimed(_) => "claimed",
            Token::Parent(_) => "parent",
            Token::Unknown(_) => "",
        }
    }

    /// Whether the task has the field the token names.
    fn matches(&self, task: &Task) -> bool {
        match self {
            Token::Status(statuses) => statuses.contains(&task.status),
            Token::Priority(priority) => task.meta.priority == *priority,
            Token::Effort(effort) => task.meta.effort == *effort,
            Token::Kind(kind) => task.kind == *kind,
            Token::Tag(tag) => task.meta.tags.iter().any(|other| other == tag),
            Token::Claimed(claimed) => task.claim.is_some() == *claimed,
            Token::Parent(parent) => task.parent.as_deref() == Some(parent.as_str()),
            Token::Unknown(_) => false,
        }
    }
}

/// Whether a plain word, in lower case, is part of the task's id, title or a
/// tag.
fn word_matches(task: &Task, word: &str) -> bool {
    task.id.to_lowercase().contains(word)
        || task.title.to_lowercase().contains(word)
        || task
            .meta
            .tags
            .iter()
            .any(|tag| tag.to_lowercase().contains(word))
}

/// The token a `key:value` word is. The key is read without case; a value
/// the key does not take is an unknown value.
fn token(key: &str, value: &str) -> Token {
    let key = key.to_lowercase();
    let value_error = |key: &'static str| {
        Token::Unknown(Error::Value {
            key,
            value: value.to_string(),
        })
    };
    let lower = value.to_lowercase();
    match key.as_str() {
        "status" => {
            let statuses: Vec<Status> = STATUSES
                .into_iter()
                .filter(|status| status_named(status.as_str(), &lower))
                .collect();
            if statuses.is_empty() {
                value_error("status")
            } else {
                Token::Status(statuses)
            }
        }
        // The spec names `medium`: the model's middle value is `normal`.
        "priority" if lower == "medium" => Token::Priority(Priority::Normal),
        "priority" => PRIORITIES
            .into_iter()
            .find(|priority| priority.as_str() == lower)
            .map_or_else(|| value_error("priority"), Token::Priority),
        "effort" => EFFORTS
            .into_iter()
            .find(|effort| effort.as_str().to_lowercase() == lower)
            .map_or_else(|| value_error("effort"), Token::Effort),
        "kind" => KINDS
            .into_iter()
            .find(|kind| kind.as_str() == lower)
            .map_or_else(|| value_error("kind"), Token::Kind),
        "tag" if !value.is_empty() => Token::Tag(value.to_string()),
        "parent" if !value.is_empty() => Token::Parent(value.to_string()),
        "tag" => value_error("tag"),
        "parent" => value_error("parent"),
        "claimed" => match lower.as_str() {
            "yes" => Token::Claimed(true),
            "no" => Token::Claimed(false),
            _ => value_error("claimed"),
        },
        _ => Token::Unknown(Error::Key(key)),
    }
}

/// Whether a status, by name, is named by the value: the name itself, a
/// prefix of it, or one of its hyphen-separated words. `review` names
/// `code-review`; `d` names `draft` and `done`.
fn status_named(name: &str, value: &str) -> bool {
    !value.is_empty() && (name.starts_with(value) || name.split('-').any(|part| part == value))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A task with an id, a title, a status, a priority, an effort and a kind.
    fn task(id: &str, title: &str, status: Status) -> Task {
        let mut task = Task::new(id, title);
        task.status = status;
        task
    }

    fn parsed(text: &str) -> Filter {
        Filter::parse(text)
    }

    fn matches(text: &str, task: &Task) -> bool {
        parsed(text).matches(task)
    }

    #[test]
    fn empty_text_is_no_filter_and_matches_every_task() {
        let filter = parsed("");
        assert!(filter.is_empty());
        assert!(filter.unknown().is_none());
        assert!(filter.matches(&task("TSK01", "anything", Status::Draft)));
        assert!(parsed("   \t ").is_empty(), "blank text is no filter");
    }

    #[test]
    fn a_plain_word_matches_the_id_the_title_or_a_tag_in_any_case() {
        let mut tagged = task("TSK02", "Nothing to see", Status::Draft);
        tagged.meta.tags = vec!["Parser".to_string()];
        assert!(matches("tsk02", &tagged), "the id, in lower case");
        assert!(matches("NOTHING", &tagged), "the title, in capitals");
        assert!(matches("parser", &tagged), "a tag, in lower case");
        assert!(matches("ser", &tagged), "a substring of a tag");
        assert!(!matches("colour", &tagged));
    }

    #[test]
    fn every_word_must_match() {
        let task = task("TSK03", "render the footer", Status::Groomed);
        assert!(matches("render footer", &task));
        assert!(!matches("render colour", &task), "one word missed");
    }

    #[test]
    fn the_status_token_takes_a_name_a_prefix_or_a_word_of_one() {
        let review = task("TSK04", "a", Status::CodeReview);
        assert!(matches("status:code-review", &review));
        assert!(matches("status:review", &review), "a word of the name");
        assert!(matches("status:code", &review), "a prefix");
        assert!(matches("status:CODE", &review), "any case");
        assert!(!matches("status:draft", &review));
        assert_eq!(
            parsed("status:d").tokens,
            vec![Token::Status(vec![Status::Draft, Status::Done])],
            "an ambiguous prefix names every status it starts"
        );
    }

    #[test]
    fn the_priority_token_takes_the_model_values() {
        let mut urgent = task("TSK05", "a", Status::Draft);
        urgent.meta.priority = Priority::Urgent;
        assert!(matches("priority:urgent", &urgent));
        assert!(matches("priority:URGENT", &urgent));
        assert!(!matches("priority:low", &urgent));
        assert_eq!(
            parsed("priority:high").tokens,
            vec![Token::Priority(Priority::High)]
        );
        assert_eq!(
            parsed("priority:normal").tokens,
            vec![Token::Priority(Priority::Normal)]
        );
    }

    #[test]
    fn the_priority_token_takes_medium_as_the_middle_value() {
        // The spec table lists `medium`; the model has `normal` in its place.
        let mut normal = task("TSK22", "a", Status::Draft);
        normal.meta.priority = Priority::Normal;
        assert!(matches("priority:medium", &normal));
        assert!(matches("priority:MEDIUM", &normal));
        assert!(!matches("priority:high", &normal));
        assert_eq!(parsed("priority:medium").unknown(), None);
    }

    #[test]
    fn the_effort_token_takes_the_letters() {
        let mut large = task("TSK06", "a", Status::Draft);
        large.meta.effort = Effort::Large;
        assert!(matches("effort:L", &large));
        assert!(matches("effort:l", &large));
        assert!(!matches("effort:S", &large));
    }

    #[test]
    fn the_kind_token_takes_the_task_kinds() {
        let mut bug = task("TSK07", "a", Status::Draft);
        bug.kind = TaskKind::Bug;
        assert!(matches("kind:bug", &bug));
        assert!(!matches("kind:feature", &bug));
        assert!(!matches("kind:task", &bug));
    }

    #[test]
    fn the_tag_token_matches_a_tag_exactly() {
        let mut task = task("TSK08", "a", Status::Draft);
        task.meta.tags = vec!["parser".to_string()];
        assert!(matches("tag:parser", &task));
        assert!(!matches("tag:pars", &task), "a tag is not a substring");
        assert!(!matches("tag:parser:x", &task), "the value keeps its colon");
    }

    #[test]
    fn the_claimed_token_says_whether_a_claim_is_held() {
        let mut claimed = task("TSK09", "a", Status::Executing);
        claimed.claim = Some(coding_crew_core::task::Claim {
            by: "builder".to_string(),
            ..Default::default()
        });
        let free = task("TSK10", "a", Status::Approved);
        assert!(matches("claimed:yes", &claimed));
        assert!(!matches("claimed:no", &claimed));
        assert!(matches("claimed:no", &free));
        assert!(!matches("claimed:yes", &free));
    }

    #[test]
    fn the_parent_token_matches_the_subtasks_of_a_task() {
        let mut child = task("TSK11", "a", Status::Draft);
        child.parent = Some("TSK03".to_string());
        assert!(matches("parent:TSK03", &child));
        assert!(!matches("parent:TSK04", &child));
        assert!(!matches("parent:TSK03", &task("TSK03", "a", Status::Draft)));
    }

    #[test]
    fn different_keys_combine_with_and() {
        let mut high = task("TSK12", "a", Status::CodeReview);
        high.meta.priority = Priority::High;
        assert!(matches("status:review priority:high", &high));
        assert!(!matches("status:review priority:low", &high));
        assert!(!matches("status:done priority:high", &high));
    }

    #[test]
    fn the_same_key_repeated_combines_with_or() {
        let review = task("TSK13", "a", Status::CodeReview);
        let remediating = task("TSK14", "a", Status::Remediating);
        let done = task("TSK15", "a", Status::Done);
        let text = "status:review status:remediating priority:normal";
        assert!(matches(text, &review));
        assert!(matches(text, &remediating));
        assert!(!matches(text, &done));
    }

    #[test]
    fn the_spec_example_reads_as_the_spec_says() {
        // status:review status:remediating priority:high: review or
        // remediating, and high.
        let mut high = task("TSK16", "a", Status::Remediating);
        high.meta.priority = Priority::High;
        let mut normal = task("TSK17", "a", Status::Remediating);
        normal.meta.priority = Priority::Normal;
        let text = "status:review status:remediating priority:high";
        assert!(matches(text, &high));
        assert!(!matches(text, &normal));
    }

    #[test]
    fn words_and_tokens_combine() {
        let mut task = task("TSK18", "the parser", Status::Approved);
        task.meta.priority = Priority::Low;
        assert!(matches("parser priority:low", &task));
        assert!(!matches("parser priority:high", &task));
    }

    #[test]
    fn an_unknown_key_is_named_and_matches_nothing() {
        let filter = parsed("colour:red");
        assert_eq!(
            filter.unknown().map(ToString::to_string).as_deref(),
            Some("unknown key: colour")
        );
        assert!(!filter.matches(&task("TSK19", "a", Status::Draft)));
    }

    #[test]
    fn an_unknown_value_is_named_and_matches_nothing() {
        let filter = parsed("priority:huge");
        assert_eq!(
            filter.unknown().map(ToString::to_string).as_deref(),
            Some("unknown priority: huge")
        );
        assert!(!filter.matches(&task("TSK20", "a", Status::Draft)));
        assert_eq!(
            parsed("status:nowhere")
                .unknown()
                .map(ToString::to_string)
                .as_deref(),
            Some("unknown status: nowhere")
        );
        assert_eq!(
            parsed("claimed:maybe")
                .unknown()
                .map(ToString::to_string)
                .as_deref(),
            Some("unknown claimed: maybe")
        );
    }

    #[test]
    fn an_unknown_token_takes_the_whole_filter_down() {
        // A token that matches nothing leaves the AND with nothing to pass,
        // whatever the other tokens and words say.
        let task = task("TSK21", "a", Status::Draft);
        assert!(matches("status:draft", &task));
        assert!(!matches("status:draft colour:red", &task));
        assert!(!matches("a status:draft priority:huge", &task));
    }

    #[test]
    fn an_empty_value_is_an_unknown_value() {
        assert!(parsed("status:").unknown().is_some());
        assert!(parsed("tag:").unknown().is_some());
        assert!(parsed("parent:").unknown().is_some());
    }

    #[test]
    fn a_word_with_a_colon_at_its_start_is_a_plain_word() {
        // Nothing before the colon names a key: the word is text.
        let filter = parsed(":x");
        assert!(filter.tokens.is_empty());
        assert_eq!(filter.words, [":x"]);
    }

    #[test]
    fn the_key_is_read_without_case() {
        assert_eq!(
            parsed("Status:Done").tokens,
            vec![Token::Status(vec![Status::Done])]
        );
    }
}
