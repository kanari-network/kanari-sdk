# Vector

Vectors are a native way to store collections of elements in Move. They are similar to arrays in other programming languages, but with a few differences. In this section, we introduce the vector type and its operations.

## Vectoe syntax

The [vector] type is defined using the [vector] keyword followed by the type of the elements in angle brackets. The type of the elements can be any valid Move type, including other vectors. Move has a vector literal syntax that allows you to create vectors using the [vector] keyword followed by square brackets containing the elements (or no elements for an empty vector).

```rust
// An empty vector of bool elements.
let empty: vector<bool> = vector[];

// A vector of u8 elements.
let v: vector<u8> = vector[10, 20, 30];

// A vector of vector<u8> elements.
let vv: vector<vector<u8>> = vector[
    vector[10, 20],
    vector[30, 40]
];
```
The vector type is a built-in type in Move, and does not need to be imported from a module. However, vector operations are defined in the std::vector module, and you need to import the module to use them.


## Vector operations

The standard library provides methods to manipulate vectors. The following are some of the most commonly used operations:

* push_back: Adds an element to the end of the vector.
* pop_back: Removes the last element from the vector.
* length: Returns the number of elements in the vector.
* is_empty: Returns true if the vector is empty.
* remove: Removes an element at a given index.


```rust
let mut v = vector[10u8, 20, 30];

assert!(v.length() == 3);
assert!(!v.is_empty());

v.push_back(40);
let last_value = v.pop_back();

assert!(last_value == 40);
```

## Destroying a Vector of non-droppable types

A vector of non-droppable types cannot be discarded. If you define a vector of types without drop ability, the vector value cannot be ignored. However, if the vector is empty, compiler requires an explicit call to destroy_empty function.

```rust 
/// A struct without `drop` ability.
struct MyDrop {}

#[test]
fun test_destroy_empty() {
    // Initialize a vector of `MyDrop` elements.
    let v = vector<MyDrop>[];

    // While we know that `v` is empty, we still need to call
    // the explicit `destroy_empty` function to discard the vector.
    v.destroy_empty();
}
```