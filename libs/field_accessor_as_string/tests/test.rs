#[cfg(test)]
mod tests {
    use field_accessor_as_string::FieldAccessorAsString;

    #[derive(Default, FieldAccessorAsString)]
    struct MyStruct {
        field1: String,
        field2: i32,
    }

    #[test]
    fn test_set_fields() {
        let mut my_struct = MyStruct::default();
        
        my_struct.set_field("field1", "hello");
        assert_eq!(my_struct.field1, "hello");

        my_struct.set_field("field2", "42");
        assert_eq!(my_struct.field2, 42);
    }

    #[test]
    fn test_set_fields_field_does_not_exist() {
        let mut my_struct = MyStruct::default();
        
        let result = my_struct.set_field("nonexistent_field", "hello");

        assert!(result.is_err());
    }

    #[test]
    fn test_get_fields() {
        let mut my_struct = MyStruct::default();
        
        my_struct.set_field("field1", "hello");
        assert_eq!(my_struct.field1, "hello");

        my_struct.set_field("field2", "42");
        assert_eq!(my_struct.field2, 42);
    }

    #[test]
    fn test_is_field() {
        let my_struct = MyStruct::default();

        assert!(my_struct.is_field("field1"));
        assert!(my_struct.is_field("field2"));
        assert!(!my_struct.is_field("nonexistent_field"));
    }
}