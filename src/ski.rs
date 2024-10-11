use std::marker::PhantomData;

pub struct C<T>(PhantomData<T>);
pub trait Combinator<T> {
    type Apply;
}

pub struct I;
pub struct K;
pub struct KInner<T>(PhantomData<T>);
pub struct S;
pub struct SInner<T>(PhantomData<T>);
pub struct SInner2<T, U>(PhantomData<T>, PhantomData<U>);
impl<T> Combinator<T> for I {
    type Apply = I;
}
impl<T> Combinator<T> for K {
    type Apply = KInner<T>;
}
impl<T, U> Combinator<U> for KInner<T> {
    type Apply = U;
}
impl<T> Combinator<T> for S {
    type Apply = SInner<T>;
}
impl<T, U> Combinator<U> for SInner<T> {
    type Apply = SInner2<T, U>;
}
impl<F, G, X> Combinator<X> for SInner2<F, G>
where
    F: Combinator<X>,
    G: Combinator<X>,
    <F as Combinator<X>>::Apply: Combinator<<G as Combinator<X>>::Apply>,
{
    type Apply = <<F as Combinator<X>>::Apply as Combinator<<G as Combinator<X>>::Apply>>::Apply;
}
macro_rules! ski {
    ($comb:ident) => { $comb };
    (($($paren:tt)+)) => {
        ski!($($paren)+)
    };
    ($l:ident $r:ident) => {
        <$l as Combinator<$r>>::Apply
    };
    ($l:ident $r:ident $($t:tt)+) => {
        <<$l as Combinator<$r>>::Apply as Combinator<ski!($($t)+)>>::Apply
    };
    ($l:ident ($($paren:tt)+)) => {
        <$l as Combinator<ski!($($paren)+)>>::Apply
    };
    (($($paren:tt)+) $r:ident) => {
        <ski!($($paren)+) as Combinator<$r>>::Apply
    };
    ($l:ident ($($paren:tt)+) $($t:tt)+) => {
        <<$l as Combinator<ski!($($paren)+)>>::Apply as Combinator<ski!($($t)+)>>::Apply
    };
    (($($paren:tt)+) $r:ident $($t:tt)+) => {
       <<ski!($($paren)+) as Combinator<$r>>::Apply as Combinator<ski!($($t)+)>>::Apply
    }
}

fn main() {
    type B = ski!(S (K S) K);
    type W = ski!(S S (S K));
    type C = ski!(S (B B S) (K K));
    type S2 = ski!(B (B W) (B B C));
    type Foo = ski!(((((S S) K) I) K) K);
    type Foo2 = ski!(((((S2 S2) K) I) K) K);
    println!(
        "{}
{}",
        std::any::type_name::<Foo>(),
        std::any::type_name::<Foo2>()
    );
}
