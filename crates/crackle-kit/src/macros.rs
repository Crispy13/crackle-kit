use paste::paste;

/// # Example
/// ```ignore
///
/// trait DataTrait {
///     fn merge(&mut self, rhs:Self);
///
///     fn process(&self, string: &mut String);
/// }
///
/// struct AData;
/// impl DataTrait for AData {
///     fn merge(&mut self, rhs:Self) {
///         // ...
///     }
///
///     fn process(&self, string: &mut String) {
///         // ...
///     }
/// }
/// struct BData;
/// impl DataTrait for BData {
///     fn merge(&mut self, rhs:Self) {
///         // ...
///     }
///
///     fn process(&self, string: &mut String) {
///         // ...
///     }
/// }
/// struct CData<'a, T> {
///     data: Vec<&'a str>,
///     num: T,
/// }
///
/// impl<'a,T> DataTrait for CData<'a, T> {
///     fn merge(&mut self, rhs:Self) {
///         // ...
///     }
///
///     fn process(&self, string: &mut String) {
///         // ...
///     }
/// }
///
/// gen_macros_to_impl_bdt_for_enum!(
///     name_alias: bdt,
///     impl_start: {
///         impl<'a, T> Data<'a, T>
///     }
///         A(AData),
///         B(BData),
///         C(CData) // note that you need to remove generics
/// );
///
/// enum Data<'a, T> {
///     A(AData),
///     B(BData),
///     C(CData<'a, T>),
/// }
///
/// impl<'a, T> DataTrait for Data<'a, T> {
///     fn merge(&mut self, rhs:Self) {
///         gen_method_for_bdt_rhs!(self, merge, rhs=rhs)
///     }
///
///     fn process(&self, string: &mut String) {
///         gen_method_for_bdt!(self, string)
///     }
/// }
///
///
/// ```
macro_rules! gen_macros_to_impl_trait_for_enum {
    (name_alias: $enum_name_alias:ident, impl_start: {$($impl_start:tt)+}, $($variant:ident($field_type:ident)),+) => {
        gen_macros_to_impl_trait_for_enum!(@impl name_alias: $enum_name_alias, impl_start: {$($impl_start)+}, $($variant($field_type)),+, @dol=$);
    };

    (@impl name_alias: $enum_name_alias:ident, impl_start: {$($impl_start:tt)+}, $($variant:ident($field_type:ident)),+, @dol=$dol:tt) => {
        // inner marro to define some array types. for example: const var_names:[&str; count!($($variant)+)] = ...;
        macro_rules! _count {
            () => (0usize);
            ( $x:tt $dol($xs:tt)* ) => (1usize + count!($dol($xs)*));
        }

        $($impl_start)+ {
            fn variant_name(&self) -> &'static str {
                match self {
                    $(
                        Self::$variant(_) => stringify!($variant),
                    )+
                }
            }
        }

        gen_macros_to_impl_trait_for_enum!(
            @method_macros $enum_name_alias, $($variant),+, @dol=$
        );
    };

    (@method_macros $enum_name_alias:ident, $($variant:ident),+, @dol=$dol:tt) => {

        paste! {
            /// # Example
            /// ```ignore
            /// fn process(&self, a:i32, b:i32) -> i32 {
            ///     gen_method_for_bdt!(self, a, b)
            /// }
            /// ```
            macro_rules! [<gen_method_for_ $enum_name_alias>] {
                ($enum_value:ident, $method_ident:ident$dol(,)? $dol($args:ident),*) => {
                    match $enum_value {
                        $(
                            Self::$variant(d) => {
                                d.$method_ident($dol($args),*)
                            }
                        ),*
                    }
                };
            }
        }

        paste! {
            /// # Example
            /// ```ignore
            /// fn merge(&self, rhs:Self, b:i32) -> i32 {
            ///     gen_method_for_bdt_rhs!(self, rhs=rhs, b)
            /// }
            /// ```
            macro_rules! [<gen_method_for_ $enum_name_alias _rhs>] {
                ($enum_value:ident, $method_ident:ident, rhs=$rhs:ident$dol(,)? $dol($args:ident),*) => {
                    match ($enum_value, $rhs) {
                        $(
                            (Self::$variant(d), Self::$variant(rhs)) => {
                                d.$method_ident(rhs, $dol($args),*)
                            }
                        ),*,
                        (lhs,rhs) => {
                            panic!("Mismatched enum variants, lhs: {}, rhs: {}", lhs.variant_name(), rhs.variant_name());
                        }
                    }
                };
            }
        }



    };
}

gen_macros_to_impl_trait_for_enum!(
    name_alias: bdt,
    impl_start: {
        impl BDT
    },
    Kmer(Kmer),
    BaseCount(BaseCount),
    FragmentSize(FragmentSize),
    FragmentSizeRatio(FragmentSizeRatio)
);

enum BDT {
    Kmer(Kmer),
    BaseCount(BaseCount),
    FragmentSize(FragmentSize),
    FragmentSizeRatio(FragmentSizeRatio),
}

impl BDTMethods for BDT {
    fn process(&self, a: i32, b: i32) {
        gen_method_for_bdt!(self, process, a, b);
    }

    fn merge(&mut self, rhs: Self) {
        gen_method_for_bdt_rhs!(self, merge, rhs = rhs);
    }

    fn ssdf(&self, a: i32, b: i32) -> String {
        gen_method_for_bdt!(self, ssdf, a, b)
    }
}

struct Kmer;
struct BaseCount;
struct FragmentSize;
struct FragmentSizeRatio;

impl BDTMethods for Kmer {
    fn process(&self, a: i32, b: i32) {
        todo!()
    }

    fn merge(&mut self, rhs: Self) {
        todo!()
    }

    fn ssdf(&self, a: i32, b: i32) -> String {
        todo!()
    }
}

impl BDTMethods for BaseCount {
    fn process(&self, a: i32, b: i32) {
        todo!()
    }

    fn merge(&mut self, rhs: Self) {
        todo!()
    }

    fn ssdf(&self, a: i32, b: i32) -> String {
        todo!()
    }
}

impl BDTMethods for FragmentSize {
    fn process(&self, a: i32, b: i32) {
        todo!()
    }

    fn merge(&mut self, rhs: Self) {
        todo!()
    }

    fn ssdf(&self, a: i32, b: i32) -> String {
        todo!()
    }
}

impl BDTMethods for FragmentSizeRatio {
    fn process(&self, a: i32, b: i32) {
        todo!()
    }

    fn merge(&mut self, rhs: Self) {
        todo!()
    }

    fn ssdf(&self, a: i32, b: i32) -> String {
        todo!()
    }
}

trait BDTMethods {
    fn process(&self, a: i32, b: i32);

    fn merge(&mut self, rhs: Self);

    fn ssdf(&self, a: i32, b: i32) -> String;
}



