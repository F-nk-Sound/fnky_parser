use lalrpop_util::lalrpop_mod;
use paste::paste;

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
                    pub fn fake() -> Self { Self(GCHandle(0)) }

                    pub extern "C" fn [< mock_ $lower_name >]($(_: $arg_ty),*) -> Self {
                        Self::fake()
                    }

                    pub fn to_ast(self) -> IFunctionAST {
                        IFunctionAST(self.0)
                    }
                }
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
        }
    };
}

#[repr(transparent)]
pub struct GCHandle(isize);

decl_gc_handle!(
    Absolute/absolute(inner: IFunctionAST),
    Add/add(lhs: IFunctionAST, rhs: IFunctionAST),
    Ceil/ceil(inner: IFunctionAST),
    Cosine/cosine(inner: IFunctionAST),
    Divide/divide(lhs: IFunctionAST, rhs: IFunctionAST),
    Exponent/exponent(base: IFunctionAST, power: IFunctionAST),
    Floor/floor(inner: IFunctionAST),
    Multiply/multiply(lhs: IFunctionAST, rhs: IFunctionAST),
    Number/number(value: f64),
    Sine/sine(inner: IFunctionAST),
    Subtract/subtract(lhs: IFunctionAST, rhs: IFunctionAST),
    Tangent/tangent(inner: IFunctionAST),
    Variable/variable(name: GCString),
    --
    new_string: extern "C" fn(*const u8, usize) -> GCString = GCString::mock_string,
);

#[repr(transparent)]
pub struct IFunctionAST(GCHandle);

#[repr(transparent)]
pub struct GCString(GCHandle);

impl GCString {
    pub fn fake() -> Self {
        Self(GCHandle(0))
    }

    pub extern "C" fn mock_string(_: *const u8, _: usize) -> Self {
        Self::fake()
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
    use crate::{parser, CtorTable};

    #[test]
    fn parse_variable() {
        let table = CtorTable::mock_table();
        let result = parser::FunctionParser::new().parse(&table, "a_12");
        assert!(
            result.is_ok(),
            "{}",
            match result {
                Ok(_) => panic!(),
                Err(err) => err,
            }
        );
    }

    #[test]
    fn parse_add() {
        let table = CtorTable::mock_table();
        let result = parser::FunctionParser::new().parse(&table, "a_1 + b_xy + 34");
        assert!(result.is_ok());
    }

    #[test]
    fn parse_complex_arith() {
        let table = CtorTable::mock_table();
        let result = parser::FunctionParser::new().parse(&table, "(5t + 4)/4 + (4t * 3)/3");
        assert!(
            result.is_ok(),
            "{}",
            match result {
                Ok(_) => panic!(),
                Err(err) => err,
            }
        );
    }

    #[test]
    fn parse_ambiguous_abs() {
        let table = CtorTable::mock_table();
        let result = parser::FunctionParser::new().parse(&table, "abs(1)abs(1)");
        assert!(
            result.is_ok(),
            "{}",
            match result {
                Ok(_) => panic!(),
                Err(err) => err,
            }
        );
    }

    #[test]
    fn parse_square_wave() {
        let table = CtorTable::mock_table();
        let result = parser::FunctionParser::new().parse(&table, "4floor(t) - 2floor(2t) + 1");
        assert!(
            result.is_ok(),
            "{}",
            match result {
                Ok(_) => panic!(),
                Err(err) => err,
            }
        );
    }

    #[test]
    fn parse_polynomial() {
        let table = CtorTable::mock_table();
        let result = parser::FunctionParser::new().parse(&table, "5x^2 + 3x - 1");
        assert!(
            result.is_ok(),
            "{}",
            match result {
                Ok(_) => panic!(),
                Err(err) => err,
            }
        );
    }

    #[test]
    fn parse_special_constants() {
        let table = CtorTable::mock_table();
        let result = parser::FunctionParser::new().parse(&table, "pi + e");
        assert!(
            result.is_ok(),
            "{}",
            match result {
                Ok(_) => panic!(),
                Err(err) => err,
            }
        )
    }
}
