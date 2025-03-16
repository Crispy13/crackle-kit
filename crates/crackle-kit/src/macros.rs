macro_rules! gen_macros_to_impl_bdt_for_enum {
    (impl_start: {$($impl_start:tt)+}, $($variant:ident),+) => {
        $($impl_start)+ {
            fn variant_name(&self) -> &'static str {
                match self {
                    $(
                        Self::$variant(_) => stringify!($variant),
                    )+
                }
            }
        }

        gen_macros_to_impl_bdt_for_enum!(
            @method_macros $($variant),+, @dol=$
        );
    };

    (@method_macros $($variant:ident),+, @dol=$dol:tt) => {
        macro_rules! gen_method_for_bdt {
            ($enum_value:ident, $method_ident:ident, $dol($args:ident),*) => {
                match $enum_value {
                    $(
                        Self::$variant(d) => {
                            d.$method_ident($dol($args),*)
                        }
                    ),*
                }
            };
        }

        macro_rules! gen_method_for_bdt_rhs {
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
    };
}

gen_macros_to_impl_bdt_for_enum!(impl_start: {
    impl BDT
}, Kmer, BaseCount, FragmentSize, FragmentSizeRatio);

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
