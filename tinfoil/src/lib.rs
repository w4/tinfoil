#![feature(const_type_id)]

pub mod internals;

use std::any::TypeId;

pub trait Dependency<'a, C> {
    const DEPENDENCIES: &'static [TypeId];

    fn instn(context: &'a C) -> Self;
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
