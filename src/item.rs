use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};
use chrono::{Utc, DateTime};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Item {
    Event(crate::event::Event),
    Task(crate::task::Task),
}

impl Item {
    pub fn id(&self) -> &ItemId {
        match self {
            Item::Event(e) => e.id(),
            Item::Task(t) => t.id(),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Item::Event(e) => e.name(),
            Item::Task(t) => t.name(),
        }
    }

    pub fn last_modified(&self) -> DateTime<Utc> {
        match self {
            Item::Event(e) => e.last_modified(),
            Item::Task(t) => t.last_modified(),
        }
    }

    pub fn is_event(&self) -> bool {
        match &self {
            Item::Event(_) => true,
            _ => false,
        }
    }

    pub fn is_task(&self) -> bool {
        match &self {
            Item::Task(_) => true,
            _ => false,
        }
    }

    /// Returns a mutable reference to the inner Task
    ///
    /// # Panics
    /// Panics if the inner item is not a Task
    pub fn unwrap_task_mut(&mut self) -> &mut crate::task::Task {
        match self {
            Item::Task(t) => t,
            _ => panic!("Not a task"),
        }
    }

    /// Returns a reference to the inner Task
    ///
    /// # Panics
    /// Panics if the inner item is not a Task
    pub fn unwrap_task(&self) -> &crate::task::Task {
        match self {
            Item::Task(t) => t,
            _ => panic!("Not a task"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Hash, Serialize, Deserialize)]
pub struct ItemId {
    content: String,
}
impl ItemId{
    pub fn new() -> Self {
        let u = uuid::Uuid::new_v4().to_hyphenated().to_string();
        Self { content:u }
    }
}
impl Eq for ItemId {}
impl Display for ItemId {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{}", self.content)
    }
}
