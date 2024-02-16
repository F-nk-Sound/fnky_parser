use lalrpop_util::lalrpop_mod;

macro_rules! decl_gc_handle {
    ($($name:ident),*) => {
        $(
            #[repr(transparent)]
            pub struct $name(GCHandle);

            impl $name {
                pub fn fake() -> Self { Self(GCHandle(0)) }

                pub fn to_ast(self) -> IFunctionAST {
                    IFunctionAST(self.0)
                }
            }
        )*
    };
}

#[repr(transparent)]
pub struct GCHandle(isize);

decl_gc_handle!(
    Absolute,
    Add,
    Ceil,
    Cosine,
    Divide,
    Exponent,
    Floor,
    IFunctionAST,
    Multiply,
    Number,
    Sine,
    Subtract,
    Tangent,
    Variable,
    GCString
);

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

#[repr(C)]
pub struct CtorTable {
    new_number: extern "C" fn(f64) -> Number,
    new_string: extern "C" fn(*const u8, usize) -> GCString,
    new_variable: extern "C" fn(GCString) -> Variable,
}

impl CtorTable {
    pub fn new_number(&self, value: f64) -> Number {
        (self.new_number)(value)
    }

    pub fn new_string(&self, str: &str) -> GCString {
        let ptr = str.as_ptr();
        let len = str.len();
        (self.new_string)(ptr, len)
    }

    pub fn new_variable(&self, name: GCString) -> Variable {
        (self.new_variable)(name)
    }
}

#[cfg(test)]
mod tests {
    use crate::{parser, CtorTable, GCString, Number, Variable};

    extern "C" fn mock_number(_: f64) -> Number {
        Number::fake()
    }
    extern "C" fn mock_string(_: *const u8, _: usize) -> GCString {
        GCString::fake()
    }
    extern "C" fn mock_variable(_: GCString) -> Variable {
        Variable::fake()
    }

    fn mock_table() -> CtorTable {
        CtorTable {
            new_number: mock_number,
            new_string: mock_string,
            new_variable: mock_variable,
        }
    }

    #[test]
    fn parse_variable() {
        let table = mock_table();
        let result = parser::FunctionParser::new().parse(&table, "a_{12}");
        assert!(result.is_ok(), "{}", match result { Ok(_) => panic!(), Err(err) => err }); 
    }
}
