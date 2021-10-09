#![deny(clippy::all)]

use std::collections::hash_set::HashSet;
use std::rc::Rc;

mod context;
use context::{Context};

pub mod automaton;
pub use automaton::{Automaton};


pub struct AhoCorasick {
  pub automaton: Rc<Automaton>,
  pub context: Context
}

impl AhoCorasick {
  pub fn new(auto: Rc<Automaton>, return_on_first_match: bool) -> Self {
    let context: Context = Context::new(return_on_first_match);
    AhoCorasick {
      automaton: auto,
      context,
    }
  }

  pub fn clone(self) -> AhoCorasick {
    let automaton: Rc<Automaton> = Rc::clone(&self.automaton);
    let context: Context = Context::new(self.context.return_on_first_match);
    AhoCorasick {
      automaton,
      context,
    }
  }

  pub fn execute(self: &mut AhoCorasick, bytes: &[u8]) -> Option<HashSet<usize>> {
    let matches = self.automaton.execute(&mut self.context, bytes);

    if matches.is_empty() {
      return None;
    }

    Some(matches)
  }

  pub fn reset(self: &mut AhoCorasick) {
    self.context.state = 0;
  }
}

