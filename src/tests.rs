#[test]
fn test_count() {
    assert_eq!(select!(@count ()), 0);
    assert_eq!(select!(@count (_)), 1);
    assert_eq!(select!(@count (_ _ _ _ _)), 5);
    assert_eq!(select!(@count (_ _ _ _ _ _ _ _ _ _)), 10);
    assert_eq!(select!(@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)), 20);
    assert_eq!(
        select!(@count (_ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _)),
        64
    );
}

#[test]
fn test_tuple() {
    let mut nums = (0u8, 0i8, 0u16, 0i16, 0u32, 0i32, 0u64, 0i64, 0u128, 0i128, 0usize, 0usize);
    select!(@assign (_), nums, 5);
    assert_eq!(nums.1, 5);
    assert_eq!(select!(@access (_), nums), 5);

    select!(@assign (_ _ _ _ _), nums, 6);
    assert_eq!(nums.5, 6);
    assert_eq!(select!(@access (_ _ _ _ _), nums), 6);
}
