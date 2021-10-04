#![deny(clippy::all)]

#[macro_use]
extern crate napi_derive;

use std::convert::TryInto;
use std::slice;
use std::str;
use std::string::String;

use napi::{
  Env, CallContext, Property, Result,
  JsUndefined, JsBuffer, JsObject, JsNumber, JsBoolean,
};

#[cfg(all(
  any(windows, unix),
  target_arch = "x86_64",
  not(target_env = "musl"),
  not(debug_assertions)
))]
#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

// the number of characters that each state has transitions from
const MAX_CHARS: usize = 128;
const UNDEFINED: u16 = u16::MAX;

pub struct AhoCorasick {
  patterns: Vec<String>,
  max_states: u16,
  goto: Vec<[u16; MAX_CHARS]>,
  fail: Vec<u16>,
  // maybe make these JsString if we want to return them.
  out: Vec<Vec<String>>,
  // the current state, so this can be called on streaming data
  state: u16,
}

//
// call from js to match on 'x', 'yz', or '!'
//
// new AhoCorasick(Buffer.from(['x', 'yz', '!', ''].join('\x00')));
//
#[js_function(1)]
fn constructor(ctx: CallContext) -> Result<JsUndefined> {

  let pattern_buffer = &mut ctx.get::<JsBuffer>(0)?.into_value()?;
  let mut patterns: Vec<String> = vec![];

  let mut max_states: u16 = 0;
  let mut s = Vec::new();

  for ix in 0..pattern_buffer.len() {
    if pattern_buffer[ix] != 0 {
      let c: char = pattern_buffer[ix] as char;
      // this additional space is needed to make alpha case-insensitive. it
      // might be useful to accept two arguments: one case-sensitive, the
      // other case-insensitive. but for now, all text is insensitive.
      if c >= 'a' && c <= 'z' || c >= 'A' && c <= 'Z' {
        max_states += 1;
      }
      s.push(pattern_buffer[ix]);
      continue;
    }
    // it's a null, make the string if it's not zero-length.
    if s.len() > 0 {
      let pattern = String::from_utf8(s).unwrap_or_default();
      // the maximum number of states is equal to the sum of the pattern
      // lengths + 1 (for the root, or 0, state).
      // case insensitive needs to increment max states for each alpha
      // character.
      max_states += pattern.len() as u16;
      patterns.push(pattern);

      s = Vec::new();
    }
  }

  // if max_states > u16 max value return error.

  let fail: Vec<u16> = vec![0; max_states as usize + 1];

  let mut goto: Vec<[u16; MAX_CHARS]> = Vec::new();
  let mut out: Vec<Vec<String>> = Vec::new();

  for _ in 0..=max_states {
    goto.push(*Box::new([UNDEFINED; MAX_CHARS]));
    out.push(Vec::<String>::new());
  }

  // create the struct we wrap as external data.
  let mut aho = AhoCorasick {
    patterns: patterns,
    max_states: max_states,
    goto: goto,
    fail: fail,
    out: out,
    state: 0,
  };

  build_automaton(&mut aho);

  let mut this: JsObject = ctx.this_unchecked();
  ctx.env.wrap(&mut this, aho)?;

  ctx.env.get_undefined()
}

#[js_function(1)]
fn get_n(ctx: CallContext) -> Result<JsObject> {
  //let ix: u32 = ctx.get::<JsNumber>(0)?.try_into()?;
  let this: JsObject = ctx.this_unchecked();
  let aho: &mut AhoCorasick = ctx.env.unwrap(&this)?;

  let mut o: JsObject = ctx.env.create_array()?;

  for i in 0..aho.patterns.len() {
    let prop = format!("{}", i);
    let s = ctx.env.create_string_from_std(aho.patterns[i].to_string())?;
    o.set_named_property(&prop, s)?;
  }

  let max_states: i32 = aho.max_states as i32;
  o.set_named_property("maxStates", ctx.env.create_int32(max_states)?)?;

  Ok(o)
  //ctx.env.create_int32(aho.patterns.len() as i32)

  //ctx.env.create_int32(aho.n as i32)
}

fn build_automaton(aho: &mut AhoCorasick) -> u16 {
  // start with only the root state (state 0)
  let mut state_count = 1;

  // fill in the goto matrix
  for pattern in &aho.patterns {
    let bytes = pattern.as_bytes();

    let mut state: u16 = 0;

    for &byte in bytes {
      if byte >= MAX_CHARS as u8 {
        // return an error somehow.
      }
      if aho.goto[state as usize][byte as usize] == UNDEFINED {
        aho.goto[state as usize][byte as usize] = state_count;
        state_count += 1;
      }
      // save previous state so we can make transitions case-insensitive.
      let previous_state = state;
      state = aho.goto[state as usize][byte as usize];

      let extra: usize;
      if byte >= b'a' && byte <= b'z' {
        extra = (byte - (b'a' - b'A')) as usize;
      } else if byte >= b'A' && byte <= b'Z' {
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
    aho.out[state as usize].push(pattern.to_string());
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

  // iterate over all possible input byte values
  for ix in 0..MAX_CHARS {
    let byte: usize = ix as usize;
    if aho.goto[0][byte] != 0 {
      aho.fail[aho.goto[0][byte] as usize] = 0;
      queue.push(aho.goto[0][byte]);
    }
  }

  // work states in the queue
  while queue.len() > 0 {
    let state: usize = queue.remove(0) as usize;

    // for the removed state, find the failure for for all characters that
    // don't have a goto transition.
    for ix in 0..MAX_CHARS {
      let byte: usize = ix as usize;

      if (aho.goto[state][byte] == UNDEFINED) {
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
      // add string to aho.out

      // insert the next level node into the queue
      queue.push(aho.goto[state][byte]);

    }
  }

  state_count
}

#[js_function(1)]
fn suspicious(ctx: CallContext) -> Result<JsBoolean> {
  let bytes = &mut ctx.get::<JsBuffer>(0)?.into_value()?;
  let this: JsObject = ctx.this_unchecked();
  let aho: &mut AhoCorasick = ctx.env.unwrap(&this)?;

  for &b in bytes.into_iter() {
    let mut byte: u8 = b;
    // for bytes larger than the max to fail by setting to an
    // impossible value.
    if byte >= MAX_CHARS as u8 {
      byte = 0;
    }
    let mut next: u16 = aho.state;
    while aho.goto[next as usize][byte as usize] == UNDEFINED {
      next = aho.fail[next as usize];
    }

    aho.state = aho.goto[next as usize][byte as usize];

    if aho.out[aho.state as usize].len() > 0 {
      return ctx.env.get_boolean(true);
    }
  }

  ctx.env.get_boolean(false)
}

#[js_function(1)]
fn reset(ctx: CallContext) -> Result<JsUndefined> {
  let bytes = &mut ctx.get::<JsBuffer>(0)?.into_value()?;
  let this: JsObject = ctx.this_unchecked();
  let aho: &mut AhoCorasick = ctx.env.unwrap(&this)?;
  aho.state = 0;

  ctx.env.get_undefined()
}

/*
#[js_function(1)]
fn scanner_suspicious(ctx: CallContext) -> Result<JsBoolean> {
  let bytes = &mut ctx.get::<JsBuffer>(0)?.into_value()?;
  let this: JsObject = ctx.this_unchecked();
  let scanner: &mut Scanner = ctx.env.unwrap(&this)?;

  for byte in bytes.into_iter() {
    if scanner.bad_chars[*byte as usize] {
      return ctx.env.get_boolean(true);
    }
    if *byte == DASH && scanner.prev_byte == DASH {
      return ctx.env.get_boolean(true);
    }
    scanner.prev_byte = *byte;
  }
  ctx.env.get_boolean(false)
}
// */

#[module_exports]
fn init(mut exports: JsObject, env: Env) -> Result<()> {
  let aho = env.define_class("AhoCorasick", constructor, &[
    Property::new(&env, "get")?.with_method(get_n),
    Property::new(&env, "suspicious")?.with_method(suspicious),
    Property::new(&env, "reset")?.with_method(reset),
  ])?;
  exports.set_named_property("AhoCorasick", aho)?;
  Ok(())
}

/*

impl Matcher {
    pub fn new(words: js_sys::Array) -> Matcher {
        let mut g = vec![[UNDEFINED; MAX_CHARS]];
        let mut f = vec![UNDEFINED];
        let mut out = vec![Vec::<js_sys::JsString>::new()];

        build(&mut g, &mut f, &mut out, words);

        Matcher {
            g: g,
            f: f,
            out: out,
        }
    }

    pub fn run(&self, string: &js_sys::JsString) -> js_sys::Array {
        let mut state = 0;
        let results = js_sys::Array::new();
        for c in string.iter() {
            state = self.next_state(state, c);
            for word in &self.out[state] {
                results.push(word);
            }
        }

        results
    }

    fn next_state(&self, state: usize, c: u16) -> usize {
        let mut next_state = state;
        let c_id = c as usize;

        while self.g[next_state][c_id] == UNDEFINED {
            next_state = self.f[next_state] as usize;
        }

        self.g[next_state][c_id] as usize
    }
}
//
// Build the automaton
fn build(aho: AhoCorasick) {
//    g: &mut Vec<[u16; MAX_CHARS]>,
//    f: &mut Vec<u16>,
//    out: &mut Vec<Vec<js_sys::JsString>>,
//    words: js_sys::Array,
//) {
    let mut state = 0;

    for word in words.iter() {
        match wasm_bindgen::JsCast::dyn_ref::<js_sys::JsString>(&word) {
            Some(w) => {
                let mut current_state = 0;
                for c in w.iter() {
                    let c_id = c as usize;
                    // let c_id = char_id(c);
                    if g[current_state][c_id] == UNDEFINED {
                        state += 1;
                        g.push([UNDEFINED; MAX_CHARS]);
                        f.push(UNDEFINED);
                        out.push(Vec::<js_sys::JsString>::new());
                        g[current_state][c_id] = state;
                    }
                    current_state = g[current_state][c_id] as usize;
                }

                out[current_state].push(w.clone());
            }
            None => (),
        }
    }

    let mut queue: std::collections::VecDeque<usize> = std::collections::VecDeque::new();
    let mut new_words: Vec<js_sys::JsString> = Vec::new();

    for c in g[0].iter_mut() {
        if *c == UNDEFINED {
            *c = 0;
        } else {
            f[*c as usize] = 0;
            queue.push_back(*c as usize);
        }
    }

    loop {
        let state = match queue.pop_front() {
            Some(s) => s,
            None => break,
        };

        for c in 0..MAX_CHARS {
            let next = g[state][c];
            if next != UNDEFINED {
                let mut failure = f[state] as usize;
                while g[failure][c] == UNDEFINED {
                    failure = f[failure] as usize;
                }
                f[next as usize] = g[failure][c];

                let n = next as usize;
                if failure != n {
                    let f = failure as usize;
                    // Add suffix words matching
                    std::mem::swap(&mut out[f], &mut new_words);
                    for word in &new_words {
                        out[n].push(word.clone());
                    }
                    std::mem::swap(&mut out[f], &mut new_words);
                }
                queue.push_back(n);
            }
        }
    }
}
// */
