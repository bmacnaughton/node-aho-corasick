#![deny(clippy::all)]

use std::string::String;
use std::collections::hash_set::HashSet;
use std::rc::Rc;

// the number of characters that each state has transitions from
const MAX_CHARS: usize = 128;
const UNDEFINED: u16 = u16::MAX;

pub struct AhoCorasick {
  pub patterns: Vec<String>,
  pub max_states: u16,
  goto: Vec<[u16; MAX_CHARS]>,
  fail: Vec<u16>,
  // the indexes of the strings we process
  out: Vec<HashSet<usize>>,
  // the current state, so this can be called on streaming data
  state: u16,
  return_on_first_match: bool,
}

pub fn build_automaton(patterns: Vec<String>) -> Result<AhoCorasick, String> {
  let mut aho: AhoCorasick;
  match create_automaton_skeleton(patterns.clone()) {
    Err(s) => return Err(s),
    Ok(s) => aho = s,
  }

  // start with only the root state (state 0)
  let mut state_count = 1;

  // fill in the goto matrix
  for (ix, pattern) in patterns.iter().enumerate() {
    let bytes = pattern.as_bytes();

    let mut state: u16 = 0;

    for &byte in bytes {
      if aho.goto[state as usize][byte as usize] == UNDEFINED {
        aho.goto[state as usize][byte as usize] = state_count;
        state_count += 1;
      }
      // save previous state so we can make transitions case-insensitive.
      let previous_state = state;
      state = aho.goto[state as usize][byte as usize];

      let extra: usize;
      if (b'a'..=b'z').contains(&byte) {
        extra = (byte - (b'a' - b'A')) as usize;
      } else if (b'A'..=b'Z').contains(&byte) {
        extra = (byte + (b'a' - b'A')) as usize;
      } else {
        continue;
      }

      if aho.goto[previous_state as usize][extra] == UNDEFINED {
        // transition to the same state that the opposite case character
        // transitioned to.
        aho.goto[previous_state as usize][extra] = state_count - 1;
      }
    }
    aho.out[state as usize].insert(ix);
  }

  // for all root transitions that are undefined, make them transition to
  // the root.
  for ix in 0..MAX_CHARS {
    let byte: usize = ix as usize;
    if aho.goto[0][byte] == UNDEFINED {
      aho.goto[0][byte] = 0;
    }
  }

  let mut queue: Vec<u16> = Vec::<u16>::new();

  // iterate over all possible input byte values and when the root state
  // transition is to a non-root state, set the fail transition for that
  // non-root state to the root state. then add that non-root state to the
  // queue.
  for ix in 0..MAX_CHARS {
    let byte: usize = ix as usize;
    if aho.goto[0][byte] != 0 {
      aho.fail[aho.goto[0][byte] as usize] = 0;
      queue.push(aho.goto[0][byte]);
    }
  }

  // work through the states in the queue
  while !queue.is_empty() {
    let state: usize = queue.remove(0) as usize;

    // for this state, find the failure transition for all characters that
    // don't have a goto transition.
    for ix in 0..MAX_CHARS {
      let byte: usize = ix as usize;

      if aho.goto[state][byte] == UNDEFINED {
        continue;
      }

      // get the failure transition
      let mut failure: usize = aho.fail[state] as usize;

      // find the deepest node
      while aho.goto[failure][byte] == UNDEFINED {
        failure = aho.fail[failure] as usize;
      }

      // and goto that node's transition
      failure = aho.goto[failure][byte] as usize;
      aho.fail[aho.goto[state][byte] as usize] = failure as u16;

      // merge outputs
      let pattern_indexes = aho.out[failure].clone();
      for pattern_index in pattern_indexes {
        aho.out[aho.goto[state][byte] as usize].insert(pattern_index);
      }

      // insert the next level node into the queue
      queue.push(aho.goto[state][byte]);

    }
  }

  Ok(aho)
}

impl AhoCorasick {
  pub fn new(max_states: u16, patterns: Vec<String>) -> Self {
    let mut goto: Vec<[u16; MAX_CHARS]> = Vec::new();
    let fail: Vec<u16> = vec![0; max_states as usize + 1];
    let mut out: Vec<HashSet<usize>> = Vec::new();
    for _ in 0..=max_states {
      goto.push(*Box::new([UNDEFINED; MAX_CHARS]));
      out.push(HashSet::<usize>::new());
    }
    AhoCorasick {
      max_states,
      patterns,
      goto,
      fail,
      out,
      state: 0,
      return_on_first_match: false
    }
  }


  pub fn execute(self: &mut AhoCorasick, bytes: &[u8]) -> Option<HashSet<usize>> {

    let mut found = HashSet::<usize>::new();

    for &b in bytes.iter() {
      let mut byte: u8 = b;
      // force bytes larger than the max to fail by setting to an
      // impossible value.
      if byte >= MAX_CHARS as u8 {
        byte = 0;
      }
      let mut next: u16 = self.state;
      while self.goto[next as usize][byte as usize] == UNDEFINED {
        next = self.fail[next as usize];
      }

      self.state = self.goto[next as usize][byte as usize];

      let matches = &self.out[self.state as usize];
      if !matches.is_empty() {
        for pattern_index in matches.iter() {
          found.insert(*pattern_index);
        }

        // add matches to set
        if self.return_on_first_match {
          return Some(found);
        }
      }
    }

    if !found.is_empty() {
      return Some(found);
    }

    None
  }

  pub fn reset(self: &mut AhoCorasick) {
    self.state = 0;
  }
}

fn create_automaton_skeleton(patterns: Vec<String>) -> Result<AhoCorasick, String> {
  // max_states is a u32 so we it can exceed the u16::MAX value (states are
  // u16 to save a couple bytes per state/character) and we can detect
  // overflow.
  let mut max_states: u32 = 0;

  // for each pattern step through the characters and bump max_states for
  // each alpha character. all patterns are presumed to be case insensitive
  // now; if that changes then separate pattern categories will need to be
  // supplied.
  for pattern in patterns.iter() {
    // a state for each char in the pattern
    max_states += pattern.len() as u32;

    // we can iterate on bytes because utf-8 represents the ascii domain
    // natively. non-ascii characters' bytes will not match any of the
    // patterns we use.
    for c in pattern.bytes() {
      if c >= MAX_CHARS as u8 {
        return Err("patterns cannot contain non-ASCII characters".to_string());
      }
      // extra states for case insensitivity
      if ('a'..='z').contains(&(c as char)) || ('A'..='Z').contains(&(c as char)) {
        max_states += 1;
      }
    }
  }

  // if the maximum states overflow a u16 then return an error string.
  if max_states >= u16::MAX.into() {
    let msg: String = format!("total length of patterns, {}, exceeds {}", max_states, u16::MAX - 1);
    return Err(msg);
  }

  // create the struct we wrap as external data.
  let aho = AhoCorasick::new(max_states as u16, patterns);

  // and return the structure
  Ok(aho)
}
