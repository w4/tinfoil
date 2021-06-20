#![feature(const_type_id)]
#![cfg(test)]

use std::marker::PhantomPinned;
use std::mem::MaybeUninit;
use std::pin::Pin;
use tinfoil::{Dependency, Provider};
use tinfoil_macros::{Tinfoil, TinfoilContext};

pub struct MyCoolValue(pub String);

pub struct MyOtherCoolValue(pub u64);

impl Default for MyOtherCoolValue {
    fn default() -> MyOtherCoolValue {
        MyOtherCoolValue(32)
    }
}

#[derive(Tinfoil)]
pub struct CoolDependency<'a> {
    pub cool: &'a MyCoolValue,
}

#[derive(Tinfoil)]
pub struct OtherDependency<'a> {
    pub cool_dep: &'a CoolDependency<'a>,
    pub cool_value: &'a MyCoolValue,
}

#[derive(TinfoilContext)]
pub struct InjectionContext<'a> {
    pub other_dependency: MaybeUninit<OtherDependency<'a>>,
    #[tinfoil(parameter)]
    pub cool_value: MyCoolValue,
    #[tinfoil(default)]
    pub other_cool_value: MyOtherCoolValue,
    pub cool_dependency: MaybeUninit<CoolDependency<'a>>,
    pub _pin: PhantomPinned,
}

fn get_context<'a>() -> Pin<Box<InjectionContext<'a>>> {
    let cool_value = MyCoolValue("yo".to_string());
    InjectionContext::new(cool_value)
}

#[test]
fn it_works() {
    let context = get_context();

    let c: &CoolDependency = context.get();
    panic!("{}", &c.cool.0)
}
