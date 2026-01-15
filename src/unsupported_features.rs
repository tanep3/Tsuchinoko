//! Unsupported feature registry (centralized guard)

use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnsupportedFeature {
    DelStatement,
    MatchStatement,
    TypeStatement,
    GlobalStatement,
    NonlocalStatement,
    WalrusOperator,
    AsyncDef,
    AwaitExpr,
    AsyncFor,
    AsyncWith,
    YieldStatement,
    YieldFrom,
    GeneratorExpr,
    CustomContextManager,
    CustomDecorator,
    ClassMethodDecorator,
    MagicMethodIter,
    MagicMethodNext,
    MagicMethodSlots,
    MagicMethodCall,
    MagicMethodRepr,
    MagicMethodStr,
    MagicMethodGetItem,
    MagicMethodSetItem,
    MagicMethodDelItem,
    MagicMethodLen,
    MagicMethodContains,
    MultipleInheritance,
    BuiltinIter,
    BuiltinNext,
    BuiltinGetattr,
    BuiltinSetattr,
    BuiltinHasattr,
    BuiltinDelattr,
    BuiltinDir,
    BuiltinVars,
    BuiltinType,
    BuiltinIssubclass,
    BuiltinId,
    BuiltinHash,
    BuiltinFormat,
    BuiltinRepr,
}

#[derive(Debug, Clone)]
pub struct UnsupportedFeatureRegistry {
    enabled: HashSet<UnsupportedFeature>,
}

impl UnsupportedFeatureRegistry {
    pub fn new(enabled: HashSet<UnsupportedFeature>) -> Self {
        Self { enabled }
    }

    pub fn is_enabled(&self, feature: UnsupportedFeature) -> bool {
        self.enabled.contains(&feature)
    }

    pub fn enable(&mut self, feature: UnsupportedFeature) {
        self.enabled.insert(feature);
    }

    pub fn disable(&mut self, feature: UnsupportedFeature) {
        self.enabled.remove(&feature);
    }
}

impl Default for UnsupportedFeatureRegistry {
    fn default() -> Self {
        let enabled: HashSet<UnsupportedFeature> = [
            UnsupportedFeature::DelStatement,
            UnsupportedFeature::MatchStatement,
            UnsupportedFeature::TypeStatement,
            UnsupportedFeature::GlobalStatement,
            UnsupportedFeature::NonlocalStatement,
            UnsupportedFeature::WalrusOperator,
            UnsupportedFeature::AsyncDef,
            UnsupportedFeature::AwaitExpr,
            UnsupportedFeature::AsyncFor,
            UnsupportedFeature::AsyncWith,
            UnsupportedFeature::YieldStatement,
            UnsupportedFeature::YieldFrom,
            UnsupportedFeature::CustomContextManager,
            UnsupportedFeature::CustomDecorator,
            UnsupportedFeature::ClassMethodDecorator,
            UnsupportedFeature::MagicMethodIter,
            UnsupportedFeature::MagicMethodNext,
            UnsupportedFeature::MagicMethodSlots,
            UnsupportedFeature::MagicMethodCall,
            UnsupportedFeature::MagicMethodRepr,
            UnsupportedFeature::MagicMethodStr,
            UnsupportedFeature::MagicMethodGetItem,
            UnsupportedFeature::MagicMethodSetItem,
            UnsupportedFeature::MagicMethodDelItem,
            UnsupportedFeature::MagicMethodLen,
            UnsupportedFeature::MagicMethodContains,
            UnsupportedFeature::MultipleInheritance,
            UnsupportedFeature::BuiltinIter,
            UnsupportedFeature::BuiltinNext,
            UnsupportedFeature::BuiltinGetattr,
            UnsupportedFeature::BuiltinSetattr,
            UnsupportedFeature::BuiltinHasattr,
            UnsupportedFeature::BuiltinDelattr,
            UnsupportedFeature::BuiltinDir,
            UnsupportedFeature::BuiltinVars,
            UnsupportedFeature::BuiltinType,
            UnsupportedFeature::BuiltinIssubclass,
            UnsupportedFeature::BuiltinId,
            UnsupportedFeature::BuiltinHash,
            UnsupportedFeature::BuiltinFormat,
            UnsupportedFeature::BuiltinRepr,
        ]
        .into_iter()
        .collect();
        Self { enabled }
    }
}
