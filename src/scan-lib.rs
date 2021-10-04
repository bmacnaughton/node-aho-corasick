#![deny(clippy::all)]

#[macro_use]
extern crate napi_derive;

use std::convert::TryInto;

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

//+++++++++++++++++++++++++++++++++
// class-based approach
//---------------------------------
struct Scanner {
  bad_chars: [bool; 256],
  prev_byte: u8
}

const DASH: u8 = '-' as u8;

#[js_function(1)]
fn scanner_constructor(ctx: CallContext) -> Result<JsUndefined> {
  let mut bad_chars: [bool; 256] = [false; 256];

  let stop_chars = &mut ctx.get::<JsBuffer>(0)?.into_value()?;

  for stop_char in stop_chars.into_iter() {
    bad_chars[*stop_char as usize] = true;
  }

  let mut scanner = Scanner {bad_chars: bad_chars, prev_byte: 0xFF};

  let mut this: JsObject = ctx.this_unchecked();
  ctx.env.wrap(&mut this, scanner)?;

  ctx.env.get_undefined()
}

#[js_function(1)]
fn scanner_get(ctx: CallContext) -> Result<JsBoolean> {
  let ix: u32 = ctx.get::<JsNumber>(0)?.try_into()?;
  let this: JsObject = ctx.this_unchecked();
  let scanner: &mut Scanner = ctx.env.unwrap(&this)?;

  ctx.env.get_boolean(scanner.bad_chars[ix as usize])
}

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

#[module_exports]
fn init(mut exports: JsObject, env: Env) -> Result<()> {
  let sclass = env.define_class("Scanner", scanner_constructor, &[
    Property::new(&env, "get")?.with_method(scanner_get),
    Property::new(&env, "suspicious")?.with_method(scanner_suspicious),
  ])?;
  exports.set_named_property("Scanner", sclass)?;
  Ok(())
}
