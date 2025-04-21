pub use macro_impl::*;

// #[impl_macro_for_enum(name=auto_gen_trait_a_methods)]
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


macro_rules! trait_method_for_enum {
    ($self:ident, $v:ident, $($code:tt)+) => {
        match $self {
            Self::A($v) => $($code)+,
            Self::B($v) => $($code)+,
            Self::C($v) => $($code)+,
        }
    };
}

macro_rules! impl_methods_trait_A_for_enum {
    ($($variant:ident),+) => {
        fn a(&self) -> u32 {
            match self {
                $(
                    Self::$variant(v) => v.a(),
                )+
            }
        }
        
        fn b(&self, rhs: Self) {
            std::todo!()
        }
        
        fn c(&self, a: i32, b: &str) -> String {
            std::todo!()
        }
    };
}

enum TestEnum<'a ,T> {
    A(AData),
    B(BData),
    C(CData<'a, T>),
}

impl<'a, T> A<'a> for TestEnum<'a, T> {
    type InnerData = ();
    
    impl_methods_trait_A_for_enum!(A,B,C);
}




// impl<'a, T> A<'a> for TestEnum<'a, T> {
//     type InnerData = ();
    
//     fn a(&self) -> u32 {
//         trait_method_for_enum!(self, v, v.a())
//     }
    
//     fn b(&self, rhs: Self) {
//         std::todo!()
//     }
    
//     fn c(&self, a: i32, b: &str) -> String {
//         std::todo!()
//     }
    
// }

// auto_gen_trait_a_methods!(
//     impl<'a, T> A for TestEnum<'a, T>
// );

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
