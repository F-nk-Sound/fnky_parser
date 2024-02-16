use lalrpop_util::lalrpop_mod;

macro_rules! decl_gc_handle {
    ($($name:ident),*) => {
        $(
            #[repr(transparent)]
            pub struct $name(GCHandle);

            impl $name {
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
pub unsafe extern "C" fn fnky_parse(input_ptr: *const u8, input_len: usize, table: &CtorTable) -> GCHandle {
    let input = unsafe { std::slice::from_raw_parts(input_ptr, input_len) };
    let input = std::str::from_utf8(input).unwrap();
    
    let output = parser::FunctionParser::new().parse(table, input);
    output.unwrap().0
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