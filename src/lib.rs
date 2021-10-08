#![deny(clippy::all)]

#[macro_use]
extern crate napi_derive;

use std::str;
use std::string::String;

use napi::{
  Env, CallContext, Property, Result, Either,
  JsUndefined, JsBuffer, JsObject, JsNumber, JsUnknown, JsString,
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

#[js_function(1)]
fn echo(ctx: CallContext) -> Result<JsUnknown> {
  let something: JsUnknown = ctx.get::<JsUnknown>(0)?;

  match something.get_type() {
    Ok(js_type) => {
      match js_type {
        napi::ValueType::String => {
          unsafe {
            let string = something.cast::<JsString>();
            Ok(string.into_unknown())
          }
        },
        napi::ValueType::Number => {
          unsafe {
            let num = something.cast::<JsNumber>();
            Ok(num.into_unknown())
          }
        },
        napi::ValueType::Object => {
          unsafe {
            let obj = something.cast::<JsObject>();
            Ok(obj.into_unknown())
          }
        }
        _ => Ok(ctx.env.get_undefined()?.into_unknown())
      }
    }
    Err(e) => Err(e)
  }
}

#[js_function(1)]
fn suspicious(ctx: CallContext) -> Result<JsUnknown> {
  let false_result: Result<JsUnknown> = Ok(ctx.env.get_null()?.into_unknown());

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
    Some(pattern_indexes) => {
      let mut a: JsObject = ctx.env.create_array()?;
      for (ix, pattern_index) in pattern_indexes.iter().enumerate() {
        let prop = format!("{}", ix);
        a.set_named_property(&prop, ctx.env.create_int64(*pattern_index as i64)?)?;
      }
      Ok(a.into_unknown())
    },
    None => false_result,
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
