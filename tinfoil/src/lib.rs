#![feature(const_type_id)]

pub mod internals;

pub use tinfoil_macros::{Tinfoil, TinfoilContext};

use std::any::TypeId;

pub trait Dependency {
    const DEPENDENCIES: &'static [TypeId];
}

///////////////////

pub trait Provider<'a, T> {
    fn get(&'a self) -> T;
}

// impl<'a, T: Copy, P: Provider<'a, &'a T>> Provider<'a, T> for P {
//     fn get(&self) -> T {
//         <Self as Provider<&T>>::get(self)
//     }
// }
