<div align="right">
<a href="./README.md">Versión Original</a>
</div>

# Jugando con los tipos en Rust

El sistema de tipos de Rust es, de hecho, turing-complete, suponiendo un límite de recursión infinito y memoria ilimitada. Esto es un conocimiento ampliamente difundido, pero casi nadie se detiene a preguntar *por qué* esto es así, o incluso qué se puede hacer con esta información. El objetivo de este repositorio es mostrar el verdadero potencial del sistema de tipos de Rust de maneras esotéricas.

El código en este repositorio puede parecer intimidante al principio, pero a continuación explicaré las técnicas generales utilizadas para que puedas entender el código y, tal vez, volverte más hábil en la manipulación de tipos.

## Programación Funcional

La Programación Funcional está en el núcleo del sistema de tipos de Rust, te guste o no. Toma el siguiente trait como ejemplo:

```rs
trait Frobbable {
    type Output;
}
```

Podrías pensar en el trait `Frobbable` como "cualquier cosa que implemente `Frobbable` contiene un tipo llamado `Output`, sea lo que sea", pero esa no es la única manera de interpretarlo. En la mayoría de los casos, esta es la forma más comprensible de pensar en traits, pero si quieres aprovechar al máximo el sistema de tipos, hay una mejor alternativa.

En lugar de eso, cada trait puede considerarse como una familia de funciones a nivel de tipo que produce uno o más tipos, donde cualquier implementador, `T`, es una función que se asigna a cada uno de los tipos en el trait. Es decir, cualquier tipo que implemente `Frobbable` puede considerarse como la función `λ.T::Output`. Esto se vuelve aún más útil si consideras que `T` es una estructura genérica. Si `T` es un tipo genérico que contiene parámetros `U...`, entonces `T` mismo puede convertirse en la función `λU... . <T<U...> as Frobbable>::Output` simplemente implementando un trait.

## Ejercicio: cálculo SKI

Con solo cambiar la forma en que piensas sobre los traits, puedes implementar fácilmente un lenguaje funcional simple: el cálculo SKI. El cálculo SKI es un lenguaje funcional turing-complete que utiliza 3 combinadores funcionales, `S`, `K`, e `I`, definidos como:

```
I x = x;
K x y = x;
S f g x = f x (g x);
```

Si deseas ver el código completo para una implementación de cálculo `SKI` en el sistema de tipos de Rust, puedes verlo en `src/ski.rs`. Debería ser bastante autoexplicativo después de cambiar la forma en que piensas sobre los traits (excepto por la macro, la cual no explicaré).

## La parte divertida: brainfuck

Si deseas leer el código completo para el intérprete de brainfuck, léelo en `src/bf.rs`.

Mi implementación de brainfuck en el sistema de tipos de Rust incluye las siguientes características:

- Celdas de tamaño de bit variable con wraparound
- Una cinta de memoria de tamaño variable (potencia de dos) con wraparound
- Compilación en funciones a nivel de tipo que luego pueden aplicarse a cualquier configuración de las anteriores
- Salida funcional a un buffer
- Corre en tiempo de compilación en el verificador de tipos y no en el evaluador const
- Soporta todas las instrucciones de brainfuck excepto `,` (porque quién necesita entrada, ¿verdad?)

Implementa aritmética de bits en el sistema de tipos y usa un árbol binario como cinta de memoria (por razones).

¿Por qué usar operaciones de bits en lugar de codificar valores como unarios? La razón principal son las limitaciones del sistema de traits. No pude encontrar una manera de hacer que una operación de módulo funcionara con unarios porque no supe cómo diferenciar entre "ser exactamente el módulo" y todo lo demás.

Además, con unarios, tendrías que elegir entre:

- Implementar un trait para cada valor del puntero (requiere macros difíciles de escribir y no es elegante)
- Tener una indexación de O(n) (lo cual se vuelve molesto muy rápido)

### Números

Los números (tanto el puntero como la celda de memoria) se almacenan como números binarios comenzando con el bit menos significativo:

```rs
pub struct U<const BIT: bool, U>(PhantomData<U>);
pub struct Nil;
```

donde `U` es esencialmente una lista enlazada de bits, terminada con `Nil`. Iteraciones previas del diseño usaban los tipos `B0` y `B1` en lugar de un genérico `BIT` constante, pero esto resultó ser perjudicial para el rendimiento y la legibilidad a largo plazo.

La operación de incremento y decremento se define en términos de álgebra booleana. Se pueden definir como se muestra a continuación, donde `X` es el bit actual y `C` es el carry in/out.

Incremento: `C` es inicialmente `1`

```
X = X xor C
C = XC
```

Decremento: `C` es inicialmente `0`

```
X = X xnor C
C = X + C
```

Se deja la demostración de estas operaciones como un ejercicio para el lector.

La implementación de brainfuck simplemente define manualmente estas tablas de verdad porque son bastante simples. Las iteraciones previas incluían un trait `TruthTable` y tipos que representaban las diversas operaciones, pero eso terminaba reduciendo el rendimiento a la mitad.

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
// *recorte*: Nil simplemente produce Nil, así que no necesitamos mostrarlo
```

Para representar otras propiedades de los números, tenemos los traits `HasValue` y `ZeroCheck`.

```rs
pub trait HasValue {
    const VALUE: usize;
}
pub trait ZeroCheck {
    type IsZero;
}
```

`HasValue` se usa exclusivamente por la instrucción `.` para mostrar valores al usuario, y `ZeroCheck` es usado por las instrucciones `[]` para verificar si se debe salir del cuerpo del ciclo.

### Memoria

Como se dijo antes, la cinta de memoria se implementa como un árbol binario:

```rs
pub struct T<B, C>(PhantomData<B>, PhantomData<C>);
```

donde `B` y `C` son `T<...>` o un número. Para acceder a una dirección de memoria desde el árbol, usamos un puntero, que implementa `TreeAccess<SomeTree>`. Ese trait se define como se muestra a continuación:

```rs
pub trait TreeAccess<P> {
    type Get: Inc + Dec;
    type Inc;
    type Dec;
}
```

`Get` devuelve el número en el puntero, `Inc` devuelve el árbol pero con la celda de memoria objetivo incrementada, y `Dec` devuelve el árbol con la celda de memoria objetivo decrementada. Diferenciamos entre estos dos casos para optimización. En teoría, podríamos haber hecho lo siguiente:

```rs
pub trait TreeAccess<P> {
    type Get;
    type Set<T>;
}
```

pero usar `Set<T>` basado en el resultado de `Get` requeriría que el verificador de tipos recorra el árbol nuevamente, lo cual no es ideal.

Para acceder a una dirección de memoria, simplemente iteramos sobre los bits en el puntero, seleccionando el árbol izquierdo si el bit es 0 y el árbol derecho si el bit es 1.

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

Observa cómo hay dos capas en el tipo. Esto es porque necesitamos poder distinguir entre `U<BIT1, U<BIT2, ...>>` y `U<BIT, Nil>`. Implementando los casos base, obtenemos:

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

Esta es la parte principal del intérprete. No fue tan complicado.

### Salida

La salida se almacena simplemente como una lista enlazada. Las entradas se almacenan de las más recientes a las más antiguas, por lo que necesitamos invertir la lista para imprimir la salida. El código debería ser bastante autoexplicativo.

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

### Codificación de Operaciones como Funciones

Primero, necesitamos una forma de mantener el estado. El estado del programa contiene la memoria, el puntero de datos y la lista de salida. Tenemos un trait llamado `StateAccess` para acceder a estos datos.

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

También necesitamos una forma de realizar operaciones sobre el `State`.

```rs
pub trait StateFunction<S> {
    type Apply: StateAccess;
}
```

Luego declaramos todas las operaciones que podemos realizar sobre el estado. La mayoría de estas operaciones son fáciles de implementar dado el marco anterior, excepto los bucles. Los bucles pueden implementarse usando recursión y especialización.

```rs
struct WhileNotZeroImpl<F, IsZeroChecker>(PhantomData<F>, PhantomData<IsZeroChecker>);
struct WhileNotZero<F>(PhantomData<F>);
```

El parámetro de tipo `IsZeroChecker` se usa para verificar si la celda de memoria actual es cero. Primero, implementemos el caso base cuando la celda de memoria es cero. Si la celda de memoria no es cero, simplemente devolvemos el estado actual.

```rs
impl<F, Mem, Ptr, Out> StateFunction<State<Mem, Ptr, Out>> for WhileNotZeroImpl<F, IsZero> {
    type Apply = State<Mem, Ptr, Out>;
}
```

De lo contrario, queremos seguir aplicando `F` al estado:

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

`ApplyPtr` es la memoria resultante de aplicar la función, y `ApplyMem` es similar para la memoria. `NextGet` es el valor de la celda de memoria apuntada por el puntero después de aplicar `F`, y `NextZeroCheck` es el tipo `IsZero` de `NextGet`. Los límites de los traits pueden parecer complicados, pero es simplemente un código de plantilla para poder realizar las operaciones.

Y eso es básicamente todo. Todo lo que necesitamos es una forma de componer operaciones, que es tan simple como:

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

Desafortunadamente, este intérprete de brainfuck no es muy rápido. No puede ejecutar programas largos que requieren muchas operaciones sin que el compilador se quede sin memoria o desborde el stack de alguna manera (aunque rustc usa `stacker`), sin contar el hecho de que el compilador requiere un límite de recursión finito.

Todos los benchmarks a continuación incluyen el tiempo de generación de código en modo debug según lo medido por `cargo` y fueron probados en una CPU `AMD Ryzen 5 5500U` de 4.056 GHz, sin estar enchufada. Entre cada prueba, se limpiaba la caché de cargo y los tiempos son simplemente el promedio de 5 pruebas.

rustc: `rustc 1.83.0-nightly (6f4ae0f34 2024-10-08)`
cargo: `cargo 1.83.0-nightly (ad074abe3 2024-10-04)`

### Hello World

`+++++++++++[>++++++>+++++++++>++++++++>++++>+++>+<<<<<<-]>++++++.>++.+++++++..+++.>>.>-.<<-.<.+++.------.--------.>>>+.>-.`: 0.60s  
`++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]>>.>---.+++++++..+++.>>.<-.<.+++.------.--------.>>+.>++.`: 0.71s  
`+[-->-[>>+>-----<<]<--<---]>-.>>>+.>>..+++[.>]<<<<.+++.------.<<-.>>>>+.`: 23.09s (ups)

## Posibles Optimizaciones

- Fusionar múltiples instrucciones iguales en una sola instrucción para que el incremento/decremento no ocurra múltiples veces (como la mayoría de los intérpretes)
- Reconocer patrones comunes como `[-]` (como la mayoría de los intérpretes)
- Fijar el ancho de las celdas de memoria / ancho del puntero (no)

## Por Hacer

- Implementar `,` (probablemente no lo haré)