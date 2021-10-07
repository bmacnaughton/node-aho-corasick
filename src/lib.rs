#![deny(clippy::all)]

#[macro_use]
extern crate napi_derive;

use std::str;
use std::string::String;

use napi::{
  Env, CallContext, Property, Result, Either,
  JsUndefined, JsBuffer, JsObject, JsNumber, JsUnknown, JsNull,
  Status, JsTypeError, JsRangeError,
};

mod aho_corasick;
use aho_corasick as aho;
use aho_corasick:: {
  AhoCorasick
};

#[cfg(all(
  any(windows, unix),
  target_arch = "x86_64",
  not(target_env = "musl"),
  not(debug_assertions)
))]
#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;



//
// call from js to match on 'x', 'yz', or '!'
//
// we pass a buffer with null-terminated strings in order to avoid passing an unknown
// number of string arguments or an array or object, both of which are somewhat
// clumsier to deal with. it's easy enough to wrap the javascript class constructor
// to translate.
//
// new AhoCorasick(Buffer.from(['x', 'yz', '!', ''].join('\x00')));
//
#[js_function(1)]
fn constructor(ctx: CallContext) -> Result<JsUndefined> {
  let pattern_buffer;
  match get_buffer(&ctx) {
    Some(buffer) => pattern_buffer = buffer.into_value()?,
    None => {
      return throw_not_buffer(ctx.env, ctx.env.get_undefined());
    }
  }

  let mut patterns: Vec<String> = vec![];

  let mut string_chars = Vec::new();
  for ix in 0..pattern_buffer.len() {
    if pattern_buffer[ix] != 0 {
      string_chars.push(pattern_buffer[ix]);
      continue;
    }
    // the character is a null, make a string if it's not zero-length.
    if !string_chars.is_empty() {
      let pattern = String::from_utf8(string_chars.clone()).unwrap_or_default();
      patterns.push(pattern);
      string_chars.clear();
    }
  }

  let aho: aho::AhoCorasick;

  match aho::build_automaton(patterns) {
    Ok(automaton) => aho = automaton,
    Err(text) => {
      let e = napi::Error {status: Status::InvalidArg, reason: text};
      unsafe {
        JsRangeError::from(e).throw_into(ctx.env.raw());
      }
      return ctx.env.get_undefined();
    }
  }

  let mut this: JsObject = ctx.this_unchecked();
  ctx.env.wrap(&mut this, aho)?;

  ctx.env.get_undefined()
}

/*
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
// */

#[js_function(1)]
fn echo(ctx: CallContext) -> Result<JsUnknown> {
  let something: JsUnknown = ctx.get::<JsUnknown>(0)?;

  let num = ctx.env.create_int32(42)?;
  let string = ctx.env.create_string("forty-two")?;
  match something.get_type() {
    Ok(js_type) => {
      match js_type {
        napi::ValueType::String => {
          Ok(string.into_unknown())
        },
        napi::ValueType::Number => Ok(num.into_unknown()),
        _ => Ok(ctx.env.get_undefined()?.into_unknown())
      }
    }
    //Ok(_js_type) => Ok(something.into_unknown()),
    Err(e) => Err(e)
  }
}
/*
  //let ix: u32 = ctx.get::<JsNumber>(0)?.try_into()?;
  let this: JsObject = ctx.this_unchecked();
  let aho: &mut AhoCorasick = ctx.env.unwrap(&this)?;

  let thing: JsUnknown = ctx.get::<JsUnknown>(0)?;
  let js_type: napi::ValueType = thing.get_type()?;
  let arg_type: String = format!("{}", js_type);


  let mut o: JsObject = ctx.env.create_array()?;

  for i in 0..aho.patterns.len() {
    let prop = format!("{}", i);
    let s = ctx.env.create_string_from_std(aho.patterns[i].to_string())?;
    o.set_named_property(&prop, s)?;
  }

  let max_states: i32 = aho.max_states as i32;
  o.set_named_property("maxStates", ctx.env.create_int32(max_states)?)?;
  o.set_named_property("argType", ctx.env.create_string_from_std(arg_type)?)?;

  Ok(o)
}
// */

#[js_function(1)]
fn suspicious(ctx: CallContext) -> Result<JsNumber> {
  let false_result: Result<JsNumber> = ctx.env.create_int32(0);

  let bytes;
  match get_buffer(&ctx) {
    Some(buffer) => bytes = buffer.into_value()?,
    None => {
      return throw_not_buffer(ctx.env, false_result);
    }
  }

  let this: JsObject = ctx.this_unchecked();
  let aho: &mut AhoCorasick = ctx.env.unwrap(&this)?;

  match aho.execute(&bytes) {
    Some(pattern_indexes) => ctx.env.create_int32(pattern_indexes.len() as i32),
    None => ctx.env.create_int32(0),
  }
}

#[js_function(1)]
fn reset(ctx: CallContext) -> Result<JsUndefined> {
  let this: JsObject = ctx.this_unchecked();
  let aho: &mut AhoCorasick = ctx.env.unwrap(&this)?;
  aho.reset();
  ctx.env.get_undefined()
}

fn get_buffer(ctx: &CallContext) -> Option<JsBuffer> {
  let result: Result<Either<JsBuffer, JsUndefined>> = ctx.try_get::<JsBuffer>(0);

  match result {
    Ok(maybe_buffer) => {
      let b: Option<JsBuffer> = Option::<JsBuffer>::from(maybe_buffer);
      b
    }
    // _e: { status: InvalidArg, reason: "expect Object, got: String" }
    Err(_e) => None
  }
}


fn _make_error(env: &Env, status: napi::Status, s: String) -> JsObject {
  let r = env.create_error(napi::Error {status, reason: s});
  match r {
    Err(e) => {
      // can't make an error - what are the chances we can throw an error?
      panic!("cannot make an error: {}", e);
    },
    // but neither TypeError not RangeError has no ::from for JsObject, so
    // can't really use this function.
    Ok(obj) => obj
  }
}

fn throw_not_buffer<T>(env: &Env, return_value: T) -> T {
  let msg: String = String::from("argument must be a buffer");
  let e = napi::Error {status: Status::InvalidArg, reason: msg};
  //let e = make_error(env, Status::InvalidArg, msg);
  unsafe {
    JsTypeError::from(e).throw_into(env.raw());
  };
  return_value
}

#[module_exports]
fn init(mut exports: JsObject, env: Env) -> Result<()> {
  exports.create_named_method("test", echo)?;
  let aho = env.define_class("AhoCorasick", constructor, &[
    //Property::new(&env, "get")?.with_method(get_n),
    Property::new(&env, "suspicious")?.with_method(suspicious),
    Property::new(&env, "reset")?.with_method(reset),
  ])?;
  exports.set_named_property("AhoCorasick", aho)?;
  Ok(())
}
