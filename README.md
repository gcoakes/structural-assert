# structural-assert

Inline assertions of the structural layout of a struct. This was implemented
exclusively to make it easier to implement structs which conform to a
specification which defines the start and end of a field. The specific use
case was initially to help faithfully reproduce structures from the [NVMe
spec](https://nvmexpress.org/specifications/).

## Usage

``` rust
ust structural_assert::test_structure;

#[test_structure(size = 20)]
#[repr(C, packed)]
pub struct Foo {
    #[loc(0:0)]
    pub a: u8,
    #[loc(1:1)]
    pub b: u8,
    #[loc(2:3)]
    pub c: u16,
    #[loc(4:19)]
    pub d: u128,
}
```
