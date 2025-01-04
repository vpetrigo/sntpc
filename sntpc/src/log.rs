#![allow(unused_imports)]
use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(feature = "defmt")] {
        pub(crate) use defmt::{debug, error};
    } else if #[cfg(feature = "log")] {
        pub(crate) use log::{debug, error};
    }
}
