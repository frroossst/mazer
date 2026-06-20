struct Dimensions<
    // meter
    const Length: i32, 
    // kilogram
    const Mass: i32,
    // second
    const Time: i32,
    // Ampere
    const ElectricCurrent: i32,
    //
    const Temperature: i32,
    const AmountOfSubstance: i32,
    const LuminousIntensity: i32,
>;

type Length = Dimensions<1, 0, 0, 0, 0, 0, 0>;
type Time   = Dimensions<0, 0, 1, 0, 0, 0, 0>;
type Speed  = Dimensions<1, 0, -1, 0, 0, 0, 0>;
type Force  = Dimensions<1, 1, -2, 0, 0, 0, 0>;

struct Quantity<D> {
    value: f64,
    _marker: std::marker::PhantomData<D>,
}

