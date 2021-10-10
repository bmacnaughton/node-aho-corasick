

pub struct Context {
  // the current state, so this can be called on streaming data
  pub state: u16,
  pub return_on_first_match: bool,
}

impl Context {
  pub fn new(return_on_first_match: bool) -> Self {
    Context {
      state: 0,
      return_on_first_match
    }
  }
}
