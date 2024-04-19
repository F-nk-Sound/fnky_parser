use lalrpop_util::lalrpop_mod;
use paste::paste;
use std::any::Any;
use std::sync::Mutex;

macro_rules! decl_gc_handle {
    (
        $(
            $name:ident/$lower_name:tt($($arg:ident: $arg_ty:ty),*)
        ),* $(,)?

        --

        $(
            $field_name:ident: $field_ty:ty = $field_mock:expr
        ),* $(,)?
    ) => {
        paste! {
            $(
                #[repr(transparent)]
                pub struct $name(GCHandle);

                impl $name {
                    pub extern "C" fn [< mock_ $lower_name >]($($arg: $arg_ty),*) -> Self {
                        let mut alloc = MOCK_ALLOC.lock().unwrap();
                        let index = alloc.vec.len();
                        alloc.vec.push(Box::new([< $name Data >]($($arg),*)));
                        Self(GCHandle(index.try_into().unwrap()))
                    }

                    pub fn to_ast(self) -> IFunctionAST {
                        IFunctionAST(self.0)
                    }
                }

                #[derive(Clone, Copy, Debug)]
                struct [< $name Data >] ($($arg_ty),*);
            )*

            #[repr(C)]
            pub struct CtorTable {
                $(
                    [< new_ $lower_name >]: extern "C" fn($($arg_ty),*) -> $name,
                )*
                $(
                    $field_name: $field_ty,
                )*
            }

            impl CtorTable {
                pub fn mock_table() -> CtorTable {
                    CtorTable {
                        $(
                            [< new_ $lower_name >]: $name::[< mock_ $lower_name >],
                        )*
                        $(
                            $field_name: $field_mock,
                        )*
                    }
                }

                $(
                    pub fn [< new_ $lower_name >](&self, $($arg: $arg_ty),*) -> $name {
                        (self.[< new_ $lower_name >])($($arg),*)
                    }
                )*
            }

            struct MockAllocator {
                vec: Vec<Box<dyn Any + Send>>,
            }

            impl MockAllocator {
                #[allow(dead_code)]
                fn get<T: 'static + Clone>(handle: GCHandle) -> Option<T> {
                    let alloc = MOCK_ALLOC.lock().unwrap();

                    let index: usize = handle.0.try_into().unwrap();
                    Some(alloc.vec[index].downcast_ref::<T>()?.clone())
                }
            }

            static MOCK_ALLOC: Mutex<MockAllocator> = Mutex::new(MockAllocator { vec: vec![] });
        }
    };
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct GCHandle(isize);

decl_gc_handle!(
    Absolute/absolute(inner: IFunctionAST),
    Add/add(lhs: IFunctionAST, rhs: IFunctionAST),
    Ceil/ceil(inner: IFunctionAST),
    Cosine/cosine(inner: IFunctionAST),
    Divide/divide(lhs: IFunctionAST, rhs: IFunctionAST),
    E/e(),
    Exponent/exponent(base: IFunctionAST, power: IFunctionAST),
    Floor/floor(inner: IFunctionAST),
    Modulo/modulo(lhs: IFunctionAST, rhs: IFunctionAST),
    Multiply/multiply(lhs: IFunctionAST, rhs: IFunctionAST),
    Number/number(value: f64),
    Pi/pi(),
    Sine/sine(inner: IFunctionAST),
    Subtract/subtract(lhs: IFunctionAST, rhs: IFunctionAST),
    Tangent/tangent(inner: IFunctionAST),
    Variable/variable(name: GCString),
    --
    new_string: extern "C" fn(*const u8, usize) -> GCString = GCString::mock_string,
);

#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct IFunctionAST(GCHandle);

#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct GCString(GCHandle);

impl GCString {
    pub fn fake() -> Self {
        Self(GCHandle(0))
    }

    extern "C" fn mock_string(chars: *const u8, len: usize) -> Self {
        // called from new_string alone, which is just getting raw parts of a string
        let str = unsafe { std::slice::from_raw_parts(chars, len) };
        let str = unsafe { std::str::from_utf8_unchecked(str) };

        let mut alloc = MOCK_ALLOC.lock().unwrap();
        let index = alloc.vec.len();
        alloc.vec.push(Box::new(str.to_string()));
        GCString(GCHandle(index.try_into().unwrap()))
    }
}

impl CtorTable {
    pub fn new_string(&self, str: &str) -> GCString {
        let ptr = str.as_ptr();
        let len = str.len();
        (self.new_string)(ptr, len)
    }
}

lalrpop_mod!(parser);

/// # Safety
/// - input_ptr must point to valid utf8 data and input_len must be how long this data is in bytes.
#[no_mangle]
pub unsafe extern "C" fn fnky_parse(
    input_ptr: *const u8,
    input_len: usize,
    table: &CtorTable,
) -> GCHandle {
    let input = unsafe { std::slice::from_raw_parts(input_ptr, input_len) };
    let Ok(input) = std::str::from_utf8(input) else {
        return table.new_string("invalid utf8 given to parser").0;
    };

    let output = parser::FunctionParser::new().parse(table, input);

    match output {
        Ok(result) => result.0,
        Err(err) => {
            let str = format!("{err}");
            table.new_string(&str).0
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        parser, AbsoluteData, AddData, CtorTable, DivideData, EData, ExponentData, FloorData,
        MockAllocator, ModuloData, MultiplyData, NumberData, PiData, SubtractData, VariableData,
    };

    #[test]
    fn parse_variable() {
        let table = CtorTable::mock_table();
        let result = parser::FunctionParser::new().parse(&table, "a_12").unwrap();

        let VariableData(a12) = MockAllocator::get(result.0).unwrap();
        let str: String = MockAllocator::get(a12.0).unwrap();
        assert_eq!(str, "a_12");
    }

    #[test]
    fn parse_add() {
        let table = CtorTable::mock_table();
        let result = parser::FunctionParser::new()
            .parse(&table, "a_1 + b_xy + 34")
            .unwrap();

        let AddData(lhs, rhs) = MockAllocator::get(result.0).unwrap(); // (a_1 + b_xy) + 34
        let AddData(lhs, rhs2) = MockAllocator::get(lhs.0).unwrap(); // a_1 + b_xy
        let VariableData(_) = MockAllocator::get(lhs.0).unwrap(); // a_1
        let VariableData(_) = MockAllocator::get(rhs2.0).unwrap(); // b_xy
        let NumberData(_) = MockAllocator::get(rhs.0).unwrap(); // 34
    }

    #[test]
    fn parse_complex_arith() {
        let table = CtorTable::mock_table();
        let result = parser::FunctionParser::new()
            .parse(&table, "(5t + 4)/4 + (4t * 3)/3")
            .unwrap();

        let AddData(lhs, rhs) = MockAllocator::get(result.0).unwrap(); // (5t + 4)/4 + (4t * 3)/3
        let DivideData(lhs, rhs2) = MockAllocator::get(lhs.0).unwrap(); // (5t + 4)/4
        let AddData(lhs, rhs3) = MockAllocator::get(lhs.0).unwrap(); // 5t + 4
        let MultiplyData(lhs, rhs4) = MockAllocator::get(lhs.0).unwrap(); // 5t
        let NumberData(_) = MockAllocator::get(lhs.0).unwrap(); // 5
        let VariableData(_) = MockAllocator::get(rhs4.0).unwrap(); // t
        let NumberData(_) = MockAllocator::get(rhs3.0).unwrap(); // 4
        let NumberData(_) = MockAllocator::get(rhs2.0).unwrap(); // 4
        let DivideData(lhs, rhs) = MockAllocator::get(rhs.0).unwrap(); // (4t * 3)/3
        let MultiplyData(lhs, rhs2) = MockAllocator::get(lhs.0).unwrap(); // 4t * 3
        let MultiplyData(lhs, rhs3) = MockAllocator::get(lhs.0).unwrap(); // 4t
        let NumberData(_) = MockAllocator::get(lhs.0).unwrap(); // 4
        let VariableData(_) = MockAllocator::get(rhs3.0).unwrap(); // t
        let NumberData(_) = MockAllocator::get(rhs2.0).unwrap(); // 3
        let NumberData(_) = MockAllocator::get(rhs.0).unwrap(); // 3
    }

    #[test]
    fn parse_ambiguous_abs() {
        let table = CtorTable::mock_table();
        let result = parser::FunctionParser::new()
            .parse(&table, "abs(1)abs(1)")
            .unwrap();

        let MultiplyData(lhs, rhs) = MockAllocator::get(result.0).unwrap(); // abs(1) * abs(1)
        let AbsoluteData(inner) = MockAllocator::get(lhs.0).unwrap(); // abs(1)
        let NumberData(_) = MockAllocator::get(inner.0).unwrap(); // 1
        let AbsoluteData(inner) = MockAllocator::get(rhs.0).unwrap(); // abs(1)
        let NumberData(_) = MockAllocator::get(inner.0).unwrap(); // 1
    }

    #[test]
    fn parse_square_wave() {
        let table = CtorTable::mock_table();
        let result = parser::FunctionParser::new()
            .parse(&table, "4floor(t) - 2floor(2t) + 1")
            .unwrap();

        let AddData(lhs, rhs) = MockAllocator::get(result.0).unwrap(); // 4floor(t) - 2floor(2t) + 1
        let SubtractData(lhs, rhs2) = MockAllocator::get(lhs.0).unwrap(); // 4floor(t) - 2floor(2t)
        let MultiplyData(lhs, rhs3) = MockAllocator::get(lhs.0).unwrap(); // 4floor(t)
        let NumberData(_) = MockAllocator::get(lhs.0).unwrap(); // 4
        let FloorData(inner) = MockAllocator::get(rhs3.0).unwrap(); // floor(t)
        let VariableData(_) = MockAllocator::get(inner.0).unwrap(); // t
        let MultiplyData(lhs, rhs2) = MockAllocator::get(rhs2.0).unwrap(); // 2floor(2t);
        let NumberData(_) = MockAllocator::get(lhs.0).unwrap(); // 2
        let FloorData(inner) = MockAllocator::get(rhs2.0).unwrap(); // floor(2t)
        let MultiplyData(lhs, rhs2) = MockAllocator::get(inner.0).unwrap(); // 2t
        let NumberData(_) = MockAllocator::get(lhs.0).unwrap(); // 2
        let VariableData(_) = MockAllocator::get(rhs2.0).unwrap(); // t
        let NumberData(_) = MockAllocator::get(rhs.0).unwrap(); // 1
    }

    #[test]
    fn parse_polynomial() {
        let table = CtorTable::mock_table();
        let result = parser::FunctionParser::new()
            .parse(&table, "5x^2 + 3x - 1")
            .unwrap();

        let SubtractData(lhs, rhs) = MockAllocator::get(result.0).unwrap(); // 5x^2 + 3x - 1
        let AddData(lhs, rhs2) = MockAllocator::get(lhs.0).unwrap(); // 5x^2 + 3x
        let MultiplyData(lhs, rhs3) = MockAllocator::get(lhs.0).unwrap(); // 5x^2
        let NumberData(_) = MockAllocator::get(lhs.0).unwrap(); // 5
        let ExponentData(base, power) = MockAllocator::get(rhs3.0).unwrap(); // x^2
        let VariableData(_) = MockAllocator::get(base.0).unwrap(); // x
        let NumberData(_) = MockAllocator::get(power.0).unwrap(); // 2
        let MultiplyData(lhs, rhs2) = MockAllocator::get(rhs2.0).unwrap(); // 3x
        let NumberData(_) = MockAllocator::get(lhs.0).unwrap(); // 3
        let VariableData(_) = MockAllocator::get(rhs2.0).unwrap(); // x
        let NumberData(_) = MockAllocator::get(rhs.0).unwrap(); // 1
    }

    #[test]
    fn parse_special_constants() {
        let table = CtorTable::mock_table();
        let result = parser::FunctionParser::new()
            .parse(&table, "pi + e")
            .unwrap();

        let AddData(lhs, rhs) = MockAllocator::get(result.0).unwrap(); // pi + e
        let PiData() = MockAllocator::get(lhs.0).unwrap(); // pi
        let EData() = MockAllocator::get(rhs.0).unwrap(); // e
    }

    #[test]
    fn parse_modulo() {
        let table = CtorTable::mock_table();
        let result = parser::FunctionParser::new()
            .parse(&table, "27 % 6")
            .unwrap();

        let ModuloData(lhs, rhs) = MockAllocator::get(result.0).unwrap(); // 27 % 6
        let NumberData(_) = MockAllocator::get(lhs.0).unwrap(); // 27
        let NumberData(_) = MockAllocator::get(rhs.0).unwrap(); // 6
    }

    #[test]
    fn parse_complex_modulo() {
        let table = CtorTable::mock_table();
        let result = parser::FunctionParser::new()
            .parse(&table, "27 % 2t")
            .unwrap();

        let ModuloData(lhs, rhs) = MockAllocator::get(result.0).unwrap(); // 27 % 6
        let NumberData(_) = MockAllocator::get(lhs.0).unwrap(); // 27
        let MultiplyData(lhs, rhs) = MockAllocator::get(rhs.0).unwrap(); // 2t
        let NumberData(_) = MockAllocator::get(lhs.0).unwrap(); // 2
        let VariableData(_) = MockAllocator::get(rhs.0).unwrap(); // t
    }
}
