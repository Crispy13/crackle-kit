pub use macro_impl::*;

#[impl_macro_for_enum(name=auto_gen_trait_a_methods)]
trait A<'a>: Sized {
    type InnerData: 'a;

    fn a(&self) -> u32 {
        todo!()
    }

    fn b(&self, rhs: Self) {
        todo!()
    }

    fn c(&self, a: i32, b: &str) -> String {
        todo!()
    }
}

enum TestEnum<'a ,T> {
    A(AData),
    B(BData),
    C(CData<'a, T>),
}

// auto_gen_trait_a_methods!(
//     impl<'a, T> A for TestEnum<'a, T>
// )

struct AData;

impl<'a> A<'a> for AData {
    type InnerData = ();

    fn a(&self) -> u32 {
        42
    }

    fn c(&self, a: i32, b: &str) -> String {
        format!("c: {} {}", a, b)
    }
}
struct BData;

impl<'a> A<'a> for BData {
    type InnerData = ();

    fn a(&self) -> u32 {
        42
    }

    fn c(&self, a: i32, b: &str) -> String {
        format!("c: {} {}", a, b)
    }
}
struct CData<'a, T> {
    phantom_data: std::marker::PhantomData<&'a T>,
}

impl<'a, T> A<'a> for CData<'a, T> {
    fn a(&self) -> u32 {
        42
    }

    fn c(&self, a: i32, b: &str) -> String {
        format!("c: {} {}", a, b)
    }
    
    type InnerData = ();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proc_macro_impl() {}
}
