#![recursion_limit = "2048"]
use std::marker::PhantomData;
pub struct Nil;
pub struct IsNotZero;
pub struct IsZero;

pub struct U<const BIT: bool, U>(PhantomData<U>);

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

impl<const V: bool> Inc<V> for Nil {
    type Output = Nil;
}
impl<const V: bool> Dec<V> for Nil {
    type Output = Nil;
}
pub trait HasValue {
    const VALUE: usize;
}
pub trait ZeroCheck {
    type IsZero;
}
impl<C: HasValue> HasValue for U<false, C> {
    const VALUE: usize = 2 * C::VALUE;
}
impl<C: HasValue> HasValue for U<true, C> {
    const VALUE: usize = 1 + 2 * C::VALUE;
}
impl HasValue for Nil {
    const VALUE: usize = 0;
}
impl ZeroCheck for Nil {
    type IsZero = IsZero;
}
impl<C: ZeroCheck> ZeroCheck for U<false, C> {
    type IsZero = C::IsZero;
}
impl<C: ZeroCheck> ZeroCheck for U<true, C> {
    type IsZero = IsNotZero;
}

/// A binary tree, representing the memory tape
pub struct T<B, C>(PhantomData<B>, PhantomData<C>);

pub trait TreeAccess<P> {
    type Get;
    type Inc;
    type Dec;
}

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

pub trait FilledTree<T> {
    type FilledTree;
}
impl<T> FilledTree<T> for Nil {
    type FilledTree = T;
}
impl<R, N: FilledTree<R>> FilledTree<R> for U<false, N> {
    type FilledTree = T<<N as FilledTree<R>>::FilledTree, <N as FilledTree<R>>::FilledTree>;
}

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
pub struct State<Mem, Ptr, Out>(PhantomData<Mem>, PhantomData<Ptr>, PhantomData<Out>);
struct IncPtr;
struct DecPtr;
struct IncMem;
struct DecMem;
struct WriteOutput;
struct WhileNotZeroImpl<F, IsZeroChecker>(PhantomData<F>, PhantomData<IsZeroChecker>);
struct WhileNotZero<F>(PhantomData<F>);
struct Then<F, G>(PhantomData<F>, PhantomData<G>);
struct Noop;
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
pub trait StateFunction<S> {
    type Apply: StateAccess;
}
impl<Mem, Ptr: Inc, Out> StateFunction<State<Mem, Ptr, Out>> for IncPtr {
    type Apply = State<Mem, <Ptr as Inc>::Output, Out>;
}
impl<Mem, Ptr: Dec, Out> StateFunction<State<Mem, Ptr, Out>> for DecPtr {
    type Apply = State<Mem, <Ptr as Dec>::Output, Out>;
}
impl<Mem, Ptr: TreeAccess<Mem>, Out> StateFunction<State<Mem, Ptr, Out>> for IncMem {
    type Apply = State<<Ptr as TreeAccess<Mem>>::Inc, Ptr, Out>;
}
impl<Mem, Ptr: TreeAccess<Mem>, Out> StateFunction<State<Mem, Ptr, Out>> for DecMem {
    type Apply = State<<Ptr as TreeAccess<Mem>>::Dec, Ptr, Out>;
}
impl<Mem, Ptr: TreeAccess<Mem>, Out: OutputList> StateFunction<State<Mem, Ptr, Out>>
    for WriteOutput
{
    type Apply = State<Mem, Ptr, List<<Ptr as TreeAccess<Mem>>::Get, Out>>;
}

type ApplyPtr<F, Mem, Ptr, Out> =
    <<F as StateFunction<State<Mem, Ptr, Out>>>::Apply as StateAccess>::Ptr;
type ApplyMem<F, Mem, Ptr, Out> =
    <<F as StateFunction<State<Mem, Ptr, Out>>>::Apply as StateAccess>::Mem;
type NextGet<F, Mem, Ptr, Out> =
    <ApplyPtr<F, Mem, Ptr, Out> as TreeAccess<ApplyMem<F, Mem, Ptr, Out>>>::Get;
type NextZeroCheck<F, Mem, Ptr, Out> = <NextGet<F, Mem, Ptr, Out> as ZeroCheck>::IsZero;

impl<F, Mem, Ptr, Out> StateFunction<State<Mem, Ptr, Out>> for WhileNotZeroImpl<F, IsZero> {
    type Apply = State<Mem, Ptr, Out>;
}
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
impl<F, Mem, Ptr, Out> StateFunction<State<Mem, Ptr, Out>> for WhileNotZero<F>
where
    F: StateFunction<State<Mem, Ptr, Out>>,
    Ptr: TreeAccess<Mem>,
    <Ptr as TreeAccess<Mem>>::Get: ZeroCheck,
    WhileNotZeroImpl<F, <<Ptr as TreeAccess<Mem>>::Get as ZeroCheck>::IsZero>:
        StateFunction<State<Mem, Ptr, Out>>,
{
    type Apply =
        <WhileNotZeroImpl<F, <<Ptr as TreeAccess<Mem>>::Get as ZeroCheck>::IsZero> as StateFunction<
            State<Mem, Ptr, Out>,
        >>::Apply;
}
impl<F, G, Mem, Ptr, Out> StateFunction<State<Mem, Ptr, Out>> for Then<F, G>
where
    F: StateFunction<State<Mem, Ptr, Out>>,
    G: StateFunction<F::Apply>,
{
    type Apply = <G as StateFunction<F::Apply>>::Apply;
}
type Apply<F, S> = <F as StateFunction<S>>::Apply;
impl<Mem, Ptr, Out> StateFunction<State<Mem, Ptr, Out>> for Noop {
    type Apply = State<Mem, Ptr, Out>;
}

fn get_output<T: StateAccess<Out: OutputList> + ?Sized>(buf: &mut [u8]) -> &mut [u8] {
    T::Out::write_output(buf, T::Out::LENGTH);
    &mut buf[..T::Out::LENGTH]
}
fn print_output<T: StateAccess<Out: OutputList> + ?Sized>(buf: &mut [u8]) {
    let buf = get_output::<T>(buf);
    match std::str::from_utf8(get_output::<T>(buf)) {
        Ok(o) => print!("{o}"),
        Err(_) => {
            print!("(Invalid Utf-8):\n{}", String::from_utf8_lossy(buf))
        }
    }
}

macro_rules! bf {
    () => { Noop };
    (+) => {
        IncMem
    };
    (-) => {
        DecMem
    };
    (->) => {
        Then<DecMem, IncPtr>
    };
    (<-) => {
        Then<DecPtr, DecMem>
    };
    (..) => {
        Then<WriteOutput, WriteOutput>
    };
    (>) => {
        IncPtr
    };
    (>>) => {
        Then<IncPtr, IncPtr>
    };
    (<<) => {
        Then<DecPtr, DecPtr>
    };
    (<) => {
        DecPtr
    };
    (.) => {
        WriteOutput
    };
    (,) => {
        compile_error!("input operator not supported")
    };

    ([$($toks:tt)*]) => {
        WhileNotZero<bf!($($toks)*)>
    };
    ($first:tt $($rest:tt)*) => {
        Then<bf!($first), bf!($($rest)*)>
    }

}

type ZeroedCell =
    U<false, U<false, U<false, U<false, U<false, U<false, U<false, U<false, Nil>>>>>>>>;
type PtrType = U<false, U<false, U<false, U<false, U<false, U<false, U<false, U<false, Nil>>>>>>>>;
type InitMem = <PtrType as FilledTree<ZeroedCell>>::FilledTree;
type InitState = State<InitMem, PtrType, Nil>;

fn main() {
    let mut buf = [0u8; (1 << 18)];
    type Operation = bf!(
        +++++++++++[>++++++>+++++++++>++++++++>++++>+++>+<<<<<<-]>++++++.>++.+++++++..+++.>>.>-.<<-.<.+++.------.--------.>>>+.>-.
    );

    type Result = Apply<Operation, InitState>;
    print_output::<Result>(&mut buf);
}
