const MAX_CHARS: usize = 127; // nb_chars + 1, contains all alphanumeric characters and most punctuation
const UNDEFINED: usize = 0xD800; // A reserved value in UTF-16

mod utils;

use js_sys;
use wasm_bindgen::prelude::*;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

// #[wasm_bindgen]
// extern "C" {
//     // Use `js_namespace` here to bind `console.log(..)` instead of just
//     // `log(..)`
//     #[wasm_bindgen(js_namespace = console)]
//     fn log(s: &str);

//     #[wasm_bindgen(js_namespace = console, js_name = log)]
//     fn log_u32(a: usize);
// }

#[wasm_bindgen]
pub struct Matcher {
    g: Vec<[usize; MAX_CHARS]>,
    f: Vec<usize>,
    out: Vec<Vec<js_sys::JsString>>,
}

#[wasm_bindgen]
impl Matcher {
    pub fn new(words: js_sys::Array) -> Matcher {
        // utils::set_panic_hook(); // Improve panic reporting

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
        // utils::set_panic_hook(); // Improve panic reporting
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
            next_state = self.f[next_state];
        }

        self.g[next_state][c_id]
    }
}

// Build the automaton
fn build(
    g: &mut Vec<[usize; MAX_CHARS]>,
    f: &mut Vec<usize>,
    out: &mut Vec<Vec<js_sys::JsString>>,
    words: js_sys::Array,
) {
    let mut state = 0;

    for word in words.iter() {
        match js_sys::JsString::try_from(&word) {
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
                    current_state = g[current_state][c_id];
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
            f[*c] = 0;
            queue.push_back(*c);
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
                let mut failure = f[state];
                while g[failure][c] == UNDEFINED {
                    failure = f[failure];
                }
                failure = g[failure][c];
                f[next] = failure;

                if failure != next {
                    // Add suffix words matching
                    std::mem::swap(&mut out[failure], &mut new_words);
                    for word in &new_words {
                        out[next].push(word.clone());
                    }
                    std::mem::swap(&mut out[failure], &mut new_words);
                }
                queue.push_back(next);
            }
        }
    }
}

// // Char to id mapping
// fn char_id(c: char) -> usize {
//     c as usize
// }
