# Rust Type-Level Fuckery

Rust's type system is, in fact, turing-complete, assuming an infinite recursion limit and memory. It's widely-known knowledge at this point, but no one ever stops to ask *why* this is the case, or even what we can do with this information. The goal of this repo is to show the true potential of Rust's type system in esoteric ways.

The code contained in this repo may look intimidating at first, but below I'll break down the general techniques used so you can understand the code and perhaps become more proficient at manipulating types.

## Functional Programming

Functional Programming is at the heart of Rust's type system, whether you like it or not. Take the following trait for example:

```rs
trait Frobbable {
    type Output;
}
```

You could think of the trait `Frobbable` as "whatever implements `Frobbable` contains a type called `Output`, whatever that means," but that is not the only way
to think about it. In most normal programming, this is the generally the best, most understandable way to think of traits, but if you want to make the
most the type system, there is a better alternative.

Instead, each trait can be thought of as a family of type-level functions that output one or more types, where any implementor, `T` is a function that maps to each of the types in the trait. That is, any type that implements `Frobbable` can be thought of as the function `λ.T::Output`. This becomes even more useful if you consider the case that `T` is a generic struct. If `T` is a generic type that contains parameters `U...`, then `T` itself can become the function `λU... . <T<U...> as Frobbable>::Output` simply by implementing a trait!

## Warm-up: SKI calculus

Just by changing the way you think about traits, you can easily implement a simple functional language: SKI calculus. SKI calculus is a turing-complete functional language that uses 3 functional combinators, `S`, `K`, and `I` defined as

```
I x = x;
K x y = x;
S f g x = f x (g x);
```

If you want to see the full code for a `SKI` calculus implementation in the Rust type system, you can view it in `src/ski.rs`.
It should be fairly self-explanatory after changing the way you think of traits (other than the macro, which I will not explain).

## The fun part: brainfuck
If you want to read the full code for the brainfuck interpreter, read `src/bf.rs`.

My implementation of brainfuck in the Rust type system has the following features:

- Variable bit-width wrapping cells
- A variable-sized (power of two) wrapping memory tape
- Compiles to type-level functions that can then be applied to any configuration of the above
- Working output to a buffer
- Runs at compile-time in the type checker and not the const evaluator.
- Supports all brainfuck instructions except `,` (because who needs input anyway)

It implements bitwise arithmetic in the type system and uses a binary tree as a memory tape (for reasons).

Why use bitwise operations instead of encoding values as unary? The main reason is limitations of the trait system. I could not figure out how to get a modulus operation working with unary because I couldn't figure out how to tell the difference between "being exactly the modulus" and everything else.

Also, with unary, you would have to choose between:

- Implementing a trait for each pointer value (requires hard-to-write macros and is not elegant)
- Having O(n) indexing (which gets annoying really quickly)

### Numbers

Numbers (both the pointer and memmory cell) are stored as a binary numbers starting with the least significant bit:

```rs
pub struct U<const BIT: bool, U>(PhantomData<U>);
pub struct Nil;
```

where `U` is essentially a linked list of bits, terminated with `Nil`. Previous iterations of the design used types `B0` and `B1` instead of a const generic `BIT`,
but that proved to be detrimental to performance and readability later.

Incrementing/Decrementing are defined in terms of boolean algebra. They can be defined as below where `X` is the current bit, and `C` is the carry in/out.

Incrementing: C is initially `1`

```
X = X xor C
C = XC
```

Decrementing: C is initially `0`

```
X = X xnor C
C = X + C
```

Proof of these is left as an exercise to the reader.

The brainfuck implementation simply manually implements these truth tables because they're quite simple. Previous iterations had a `TruthTable` trait,
and had types that represented the various operations, but that ended up cutting performance in half.

```rs
pub trait Inc<const CARRY_IN: bool = true> {
    type Output;
}
pub trait Dec<const CARRY_IN: bool = false> {
    type Output;
}
impl<C: Dec> Dec<false> for U<false, C> {
    type Output = U<true, <C as Dec<false>>::Output>;
}
impl<C: Dec<true>> Dec<false> for U<true, C> {
    type Output = U<false, <C as Dec<true>>::Output>;
}
impl<C: Dec<true>> Dec<true> for U<false, C> {
    type Output = U<false, <C as Dec<true>>::Output>;
}
impl<C: Dec<true>> Dec<true> for U<true, C> {
    type Output = U<true, <C as Dec<true>>::Output>;
}
impl<C: Inc<false>> Inc<false> for U<false, C> {
    type Output = U<false, <C as Inc<false>>::Output>;
}
impl<C: Inc<false>> Inc<false> for U<true, C> {
    type Output = U<true, <C as Inc<false>>::Output>;
}
impl<C: Inc<false>> Inc<true> for U<false, C> {
    type Output = U<true, <C as Inc<false>>::Output>;
}
impl<C: Inc> Inc<true> for U<true, C> {
    type Output = U<false, <C as Inc<true>>::Output>;
}
// *snip*: Nil just makes Nil, so we don't need to show that
```

For representing the other properties of numbers, we have the `HasValue` and `ZeroCheck` traits.

```rs
pub trait HasValue {
    const VALUE: usize;
}
pub trait ZeroCheck {
    type IsZero;
}
```

`HasValue` is exclusively used by the `.` instruction for outputting values to the user, and `ZeroCheck` is used by the `[]` instructions for
checking whether the body should be exited.

### Memory

As said above, the memory tape is implemented as a binary tree:

```rs
pub struct T<B, C>(PhantomData<B>, PhantomData<C>);
```

where `B` and `C` are either `T<...>` or a number. To access a memory address from the tree, we use a pointer, which implements `TreeAccess<SomeTree>`.
That trait is defined as below:

```rs
pub trait TreeAccess<P> {
    type Get: Inc + Dec;
    type Inc;
    type Dec;
}
```

`Get` returns the number at the pointer, `Inc` returns the tree but with the targetted memory cell incremented, and `Dec` returns the tree
with the targetted memory cell decremented. We differentiate between these two cases for the sake of optimization. In theory, we could have done

```rs
pub trait TreeAccess<P> {
    type Get;
    type Set<T>;
}
```

but using `Set<T>` based on the result of `Get` would require the type checker to traverse the tree again, which is less than ideal.

To access a memory address, we can simply iterate over the bits in the pointer, selecting the left tree if the bit is 0, and the right tree if the bit is 1.

```rs
impl<L, R, P, const BIT: bool> TreeAccess<T<L, R>> for U<false, U<BIT, P>>
where
    U<BIT, P>: TreeAccess<L>,
{
    type Get = <U<BIT, P> as TreeAccess<L>>::Get;
    type Inc = T<<U<BIT, P> as TreeAccess<L>>::Inc, R>;
    type Dec = T<<U<BIT, P> as TreeAccess<L>>::Dec, R>;
}

impl<L, R, P, const BIT: bool> TreeAccess<T<L, R>> for U<true, U<BIT, P>>
where
    U<BIT, P>: TreeAccess<R>,
{
    type Get = <U<BIT, P> as TreeAccess<R>>::Get;
    type Inc = T<L, <U<BIT, P> as TreeAccess<R>>::Inc>;
    type Dec = T<L, <U<BIT, P> as TreeAccess<R>>::Dec>;
}
```

Notice how there are 2 layers in the type. This is because we need to be able to tell the difference between `U<BIT1, U<BIT2, ...>>` and `U<BIT, Nil>`.
Implementing the base cases, we get:

```rs
impl<L, R> TreeAccess<T<L, R>> for U<false, Nil>
where
    L: Inc + Dec,
{
    type Get = L;
    type Inc = T<<L as Inc>::Output, R>;
    type Dec = T<<L as Dec>::Output, R>;
}
impl<L, R> TreeAccess<T<L, R>> for U<true, Nil>
where
    R: Inc + Dec,
{
    type Get = R;
    type Inc = T<L, <R as Inc>::Output>;
    type Dec = T<L, <R as Dec>::Output>;
}
```

This is the bulk of the interpreter right here. It wasn't that bad.

### Output

Output is simply stored as a linked list. Entries are stored from most youngest to oldest, so we need to reverse the list to print output. The code should be quite self-explanatory.

```rs
pub struct List<N, T: OutputList>(PhantomData<N>, PhantomData<T>);
pub trait OutputList {
    const VALUE: usize;
    const LENGTH: usize;
    type Next: OutputList;
    fn write_output(mut arr: &mut [u8], orig_len: usize) {
        assert!(arr.len() >= Self::LENGTH, "buffer not large enough");
        arr = &mut arr[0..Self::LENGTH];
        let (tail, head) = arr.split_last_mut().unwrap();
        *tail = Self::VALUE as u8;
        Self::Next::write_output(head, orig_len);
    }
}
impl OutputList for Nil {
    const LENGTH: usize = 0;
    const VALUE: usize = 0;
    type Next = Nil;
    fn write_output(_: &mut [u8], _: usize) {}
}
impl<N: HasValue, T: OutputList> OutputList for List<N, T> {
    const VALUE: usize = <N as HasValue>::VALUE;
    const LENGTH: usize = 1 + T::LENGTH;
    type Next = T;
}
```

### Encoding Operations as Functions

Firstly, we need a way to hold the state. The state of the program holds the memory, data pointer, and the output list. We have a trait called `StateAccess` for accessing this data.

```rs
pub struct State<Mem, Ptr, Out>(PhantomData<Mem>, PhantomData<Ptr>, PhantomData<Out>);
pub trait StateAccess {
    type Mem;
    type Ptr;
    type Out;
}
impl<Mem, Ptr, Out> StateAccess for State<Mem, Ptr, Out> {
    type Mem = Mem;
    type Ptr = Ptr;
    type Out = Out;
}
```

We also need a way to perform operations on the `State`

```rs
pub trait StateFunction<S> {
    type Apply: StateAccess;
}
```

Then we declare all of the operations that we can perform on the state. Most of these operations are trivial to implement given the framework above except for loops. Loops can be implemented using recursion and specialization.

```rs
struct WhileNotZeroImpl<F, IsZeroChecker>(PhantomData<F>, PhantomData<IsZeroChecker>);
struct WhileNotZero<F>(PhantomData<F>);
```

The `IsZeroChecker` type parameter is used for, as the name implies, checking if the current memory cell is zero.
Firstly, let's implement the base case for when the memory cell is zero. If the memory cell is not zero, then we simply return the current state.

```rs
impl<F, Mem, Ptr, Out> StateFunction<State<Mem, Ptr, Out>> for WhileNotZeroImpl<F, IsZero> {
    type Apply = State<Mem, Ptr, Out>;
}
```

otherwise, we want to keep applying `F` to the state:

```rs
type ApplyPtr<F, Mem, Ptr, Out> =
    <<F as StateFunction<State<Mem, Ptr, Out>>>::Apply as StateAccess>::Ptr;
type ApplyMem<F, Mem, Ptr, Out> =
    <<F as StateFunction<State<Mem, Ptr, Out>>>::Apply as StateAccess>::Mem;
type NextGet<F, Mem, Ptr, Out> =
    <ApplyPtr<F, Mem, Ptr, Out> as TreeAccess<ApplyMem<F, Mem, Ptr, Out>>>::Get;
type NextZeroCheck<F, Mem, Ptr, Out> = <NextGet<F, Mem, Ptr, Out> as ZeroCheck>::IsZero;

impl<F, Mem, Ptr, Out> StateFunction<State<Mem, Ptr, Out>> for WhileNotZeroImpl<F, IsNotZero>
where
    F: StateFunction<State<Mem, Ptr, Out>>,
    ApplyPtr<F, Mem, Ptr, Out>: TreeAccess<ApplyMem<F, Mem, Ptr, Out>>,
    NextGet<F, Mem, Ptr, Out>: ZeroCheck,
    WhileNotZeroImpl<F, NextZeroCheck<F, Mem, Ptr, Out>>: StateFunction<F::Apply>,
{
    type Apply =
        <WhileNotZeroImpl<F, NextZeroCheck<F, Mem, Ptr, Out>> as StateFunction<F::Apply>>::Apply;
}
```

`ApplyPtr` is the memory from applying the function and `ApplyMem` similar for the memory. `NextGet` is the value of the memory cell targetted by
the pointer after applying `F`, and `NextZeroCheck` is the `IsZero` type of `NextGet`. The trait bounds may look scary, but it's simply boilerplate so that we can actually perform the operations.

And that's basically all. All we need is a way to compose operations, which is as simple as:

```rs
struct Then<F, G>(PhantomData<F>, PhantomData<G>);
impl<F, G, Mem, Ptr, Out> StateFunction<State<Mem, Ptr, Out>> for Then<F, G>
where
    F: StateFunction<State<Mem, Ptr, Out>>,
    G: StateFunction<F::Apply>,
{
    type Apply = <G as StateFunction<F::Apply>>::Apply;
}
```

## Benchmarks:

Unfortunately, this brainfuck interpreter is not very fast. It cannot run longer programs that require a lot of operations without the
compiler running out of memory or somehow overflowing the stack (even though rustc uses `stacker`), disregarding the fact that the
compiler requires a finite recursion limit.

All benchmarks below include the debug mode codegen time as measured by `cargo` and was tested with a 4.056GHz `AMD Ryzen 5 5500U` CPU, not plugged in.
Between each trial, the cargo cache was cleaned and times are simply the mean of 5 trials.

rustc: `rustc 1.83.0-nightly (6f4ae0f34 2024-10-08)`
cargo: `cargo 1.83.0-nightly (ad074abe3 2024-10-04)`

### Hello World

`+++++++++++[>++++++>+++++++++>++++++++>++++>+++>+<<<<<<-]>++++++.>++.+++++++..+++.>>.>-.<<-.<.+++.------.--------.>>>+.>-.`: 0.60s
`++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]>>.>---.+++++++..+++.>>.<-.<.+++.------.--------.>>+.>++.`: 0.71s
`+[-->-[>>+>-----<<]<--<---]>-.>>>+.>>..+++[.>]<<<<.+++.------.<<-.>>>>+.`: 23.09s (yikes)

## Potential Optimizations

- Coalesce multiple of the same instruction into a single instruction so incrementing/decrementing doesn't happen multiple times (like most interpreters)
- Recognize common patterns like `[-]` (like most interpreters)
- Fix the memory cell width / pointer width (no)

## Todo

- Implement `,` (probably won't)
