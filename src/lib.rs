#![deny(clippy::all)]

#[macro_use]
extern crate napi_derive;

use std::str;
use std::string::String;
use std::rc::Rc;

use napi::{
  Env, CallContext, Property, Result, Either,
  JsUndefined, JsBuffer, JsObject, JsNumber, JsUnknown, JsString,
  Status, JsTypeError, JsRangeError,
};

extern crate aho_corasick;
use aho_corasick::AhoCorasick;


#[cfg(all(
  any(windows, unix),
  target_arch = "x86_64",
  not(target_env = "musl"),
  not(debug_assertions)
))]
#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;



//fn get_arg<'env> (ctx: &'env CallContext) -> Result<&'env ArgType<'env>>
/// JavaScript-callable constructor to create an Aho-Corasick machine
#[js_function(1)]
fn constructor(ctx: CallContext) -> Result<JsUndefined> {
  let mut this: JsObject = ctx.this_unchecked();

  // decode the argument
  let something = ctx.get::<JsUnknown>(0)?;
  let arg_type = something.get_type();

  if let Err(e) = arg_type {
    return Err(e);
  }

  let object: JsObject;
  unsafe {
    object = something.cast::<JsObject>();
  }
  if !object.is_buffer()? {
    let aho: &mut AhoCorasick = ctx.env.unwrap(&object)?;
    let clone = aho.clone();
    ctx.env.wrap(&mut this, clone)?;
    return ctx.env.get_undefined();
  }

  let buffer: JsBuffer;
  unsafe {
    buffer = something.cast::<JsBuffer>();
  }
  let pattern_buffer = buffer.into_value()?;

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

  let automaton: AhoCorasick;
  match AhoCorasick::new(patterns, false) {
    Ok(auto) => automaton = auto,
    Err(text) => {
      let e = napi::Error {status: Status::InvalidArg, reason: text};
      unsafe {
        JsRangeError::from(e).throw_into(ctx.env.raw());
      }
      return ctx.env.get_undefined();
    }
  }

  ctx.env.wrap(&mut this, automaton)?;

  ctx.env.get_undefined()
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

/// get_buffer tries to convert the first javascript argument as a buffer.
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

/// currently unused function because neither TypError nor RangeError implement
/// from(JsObject).
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

/// throw a TypeError Invalid Arg "argument must be a buffer" error
fn throw_not_buffer<T>(env: &Env, return_value: T) -> T {
  let msg: String = String::from("argument must be a buffer");
  let e = napi::Error {status: Status::InvalidArg, reason: msg};
  unsafe {
    JsTypeError::from(e).throw_into(env.raw());
  };
  return_value
}

#[module_exports]
fn init(mut exports: JsObject, env: Env) -> Result<()> {
  let aho = env.define_class("AhoCorasick", constructor, &[
    Property::new(&env, "suspicious")?.with_method(suspicious),
    Property::new(&env, "reset")?.with_method(reset),
  ])?;
  exports.set_named_property("AhoCorasick", aho)?;

  exports.create_named_method("test", echo)?;
  exports.create_named_method("info", info)?;
  Ok(())
}


/// echo is a test function to play with things while in development. right now
/// it shows how to return different types by converting them from/to JsUnknown.
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
fn info(ctx: CallContext) -> Result<JsObject> {
  let this: JsObject = ctx.get::<JsObject>(0)?;
  //let this: JsObject = ctx.this_unchecked();

  let aho: &mut AhoCorasick = ctx.env.unwrap(&this)?;

  let mut o: JsObject = ctx.env.create_object()?;
  let state = ctx.env.create_int32(aho.context.state as i32)?;
  let ref_count = ctx.env.create_int32(Rc::strong_count(&aho.automaton) as i32)?;

  o.set_named_property("state", state)?;
  o.set_named_property("refCount", ref_count)?;

  Ok(o)

  //if !this.has_property("suspicious")? {
  //  return ctx.env.create_int32(-1);
  //}

  //ctx.env.get_boolean(this.has_property("suspicious")?)

  // get_all_property_names is not in v1 of napi-rs.
  //let names: JsObject = this.get_all_property_names(
  //  napi::KeyCollectionMode::OwnOnly,
  //  napi::KeyFilter::AllProperties,
  //  napi::KeyConversion::NumbersToStrings
  //);

  //let aho: &mut AhoCorasick = ctx.env.unwrap(&this)?;
  //ctx.env.create_int32(aho.context.state as i32)
}
