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
    Variable
);

lalrpop_mod!(parser);

/// # Safety
/// - input_ptr must point to valid utf8 data and input_len must be how long this data is in bytes.
#[no_mangle]
pub unsafe extern "C" fn fnky_parse(input_ptr: *const u8, input_len: usize, table: &CtorTable) -> GCHandle {
    let input = unsafe { std::slice::from_raw_parts(input_ptr, input_len) };
    let input = std::str::from_utf8(input).unwrap();
    
    let output = parser::NumberParser::new().parse(table, input);
    output.unwrap().0
}

#[repr(C)]
pub struct CtorTable {
    new_number: extern "C" fn(f64) -> Number,
}

impl CtorTable {
    pub fn new_number(&self, value: f64) -> Number {
        (self.new_number)(value)
    }
}