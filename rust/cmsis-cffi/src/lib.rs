#[macro_use]
extern crate slog;
extern crate cmsis_update;
extern crate failure;
extern crate pack_index as pi;
extern crate pdsc as pack_desc;
extern crate slog_async;
extern crate slog_term;
extern crate utils as cmsis_utils;

macro_rules! with_from_raw {
    (let $boxed:ident = $ptr:ident, $block:block) => {{
        let $boxed = unsafe { Box::from_raw($ptr) };
        let ret = $block;
        Box::into_raw($boxed);
        ret
    }};
    (let mut $boxed:ident = $ptr:ident, $block:block) => {{
        let mut $boxed = unsafe { Box::from_raw($ptr) };
        let ret = $block;
        Box::into_raw($boxed);
        ret
    }};
}
#[macro_use]
pub mod utils;

pub mod pack;
pub mod pack_index;
pub mod pdsc;
