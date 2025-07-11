use openzt_store_macro::StoreSkipArrays;

#[derive(StoreSkipArrays, Debug)]
#[repr(C)]
struct MyStruct {
    id: u32,
    name: String,
    data: [u8; 16],  // This array field will be skipped
    value: f32,
    buffer: [i32; 8], // This array field will also be skipped
}

#[derive(StoreSkipArrays, Debug)]
struct TupleStruct(u32, [u8; 4], String); // The array at index 1 will be skipped

fn main() {
    println!("This example demonstrates the StoreSkipArrays derive macro.");
    println!("When implementing Store trait:");
    println!("- Regular fields (id, name, value) will be stored/loaded");
    println!("- Array fields (data, buffer) will be skipped during store");
    println!("- Array fields will be set to Default::default() during load");
}