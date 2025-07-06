use retour_utils::hook_module;

#[hook_module("some_lib.dll")]
mod lua {
    pub fn left_alone(foo: i32) -> i32 {
        foo
    }

    pub const DATA1: usize = 4;
    pub static DATA2: usize = 2;
}
// needed for trybuild
fn main() {
    use lua::*;
    // won't run, but will verify data types are kept consistent
    let _: usize = DATA1;
    let _: usize = DATA2;
    let _: fn(i32) -> i32 = left_alone; 
}