#![deny(clippy::all)]

#[macro_use]
extern crate napi_derive;

use std::str;
use std::string::String;

use napi::{
  Env, CallContext, Property, Result, Either,
  JsUndefined, JsBuffer, JsObject, JsBoolean,
  Status, JsTypeError, JsRangeError,
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

  let mut max_states: u32 = 0;
  let mut s = Vec::new();

  for ix in 0..pattern_buffer.len() {
    if pattern_buffer[ix] != 0 {
      let c: char = pattern_buffer[ix] as char;
      // this additional space is needed to make alpha case-insensitive. it
      // might be useful to accept two arguments: one case-sensitive, the
      // other case-insensitive. but for now, all text is insensitive.
      if ('a'..='z').contains(&c) || ('A'..'Z').contains(&c) {
      //if c >= 'a' && c <= 'z' || c >= 'A' && c <= 'Z' {
        max_states += 1;
      }
      s.push(pattern_buffer[ix]);
      continue;
    }
    // it's a null, make the string if it's not zero-length.
    if !s.is_empty() {
      let pattern = String::from_utf8(s).unwrap_or_default();
      // the maximum number of states is equal to the sum of the pattern
      // lengths + 1 (for the root, or 0, state).
      // case insensitive needs to increment max states for each alpha
      // character.
      max_states += pattern.len() as u32;
      patterns.push(pattern);

      s = Vec::new();
    }
  }

  // if the maximum states overflow a u16 then throw a javascript error.
  if max_states >= u16::MAX.into() {
    let msg: String = format!("total length of patterns, {}, exceeds {}", max_states, u16::MAX - 1);
    let e = napi::Error {status: Status::InvalidArg, reason: msg};
    unsafe {
      JsRangeError::from(e).throw_into(ctx.env.raw());
    }
    return ctx.env.get_undefined();
  }

  // allocate the data for the state machine
  let fail: Vec<u16> = vec![0; max_states as usize + 1];
  let mut goto: Vec<[u16; MAX_CHARS]> = Vec::new();
  let mut out: Vec<Vec<String>> = Vec::new();

  for _ in 0..=max_states {
    goto.push(*Box::new([UNDEFINED; MAX_CHARS]));
    out.push(Vec::<String>::new());
  }

  // create the struct we wrap as external data.
  let mut aho = AhoCorasick {
    patterns,
    max_states: max_states as u16,
    goto,
    fail,
    out,
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
      if (b'a'..=b'z').contains(&byte) {
      //if byte >= b'a' && byte <= b'z' {
        extra = (byte - (b'a' - b'A')) as usize;
      } else if (b'A'..=b'Z').contains(&byte) {
      //} else if byte >= b'A' && byte <= b'Z' {
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
  while !queue.is_empty() {
    let state: usize = queue.remove(0) as usize;

    // for the removed state, find the failure for for all characters that
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
      // add string to aho.out

      // insert the next level node into the queue
      queue.push(aho.goto[state][byte]);

    }
  }

  state_count
}

#[js_function(1)]
fn suspicious(ctx: CallContext) -> Result<JsBoolean> {
  //type MaybeBuffer = Either<JsBuffer, JsUndefined>;

  let result: Result<Either<JsBuffer, JsUndefined>> = ctx.try_get::<JsBuffer>(0);
  let bytes;
  let buf;

  //napi::Either<JsBuffer, JsUndefined>

  match result {
    Ok(maybe_buffer) => {
      let b: Option<JsBuffer> = Option::<JsBuffer>::from(maybe_buffer);
      match b {
        Some(bf) => buf = bf,
        None => {
          //throw_invalid_arg(&ctx.env);
          let msg: String = String::from("argument must be a buffer");
          let e = napi::Error {status: Status::InvalidArg, reason: msg};
          unsafe {
            JsTypeError::from(e).throw_into(ctx.env.raw());
          }
          return ctx.env.get_boolean(false);
        }
      }
    }
    Err(e) => {
      let msg: String = String::from("argument must be a buffer");
      let e = napi::Error {status: Status::InvalidArg, reason: msg};
      unsafe {
        JsTypeError::from(e).throw_into(ctx.env.raw());
      }
      return ctx.env.get_boolean(false);
    }
  }
  bytes = buf.into_value()?;

  //let bytes = &mut ctx.try_get::<JsBuffer>(0)?.into_value()?;
  let this: JsObject = ctx.this_unchecked();
  let aho: &mut AhoCorasick = ctx.env.unwrap(&this)?;

  for &b in bytes.iter() {
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

    if !aho.out[aho.state as usize].is_empty() {
      return ctx.env.get_boolean(true);
    }
  }

  ctx.env.get_boolean(false)
}

#[js_function(1)]
fn reset(ctx: CallContext) -> Result<JsUndefined> {
  let this: JsObject = ctx.this_unchecked();
  let aho: &mut AhoCorasick = ctx.env.unwrap(&this)?;
  aho.state = 0;

  ctx.env.get_undefined()
}


fn make_error(env: Env, status: napi::Status, s: String) -> Result<JsObject> {
  env.create_error(napi::Error {status, reason: s})
}

/*
fn throw_invalid_arg(env: Env, status: napi::Status, s: String) {
  let error: Result<JsObject> = make_error(env, status, s);

  unsafe {
    JsTypeError::from(error).throw_into(env.raw());
  }
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
