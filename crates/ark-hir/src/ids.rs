macro_rules! define_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(pub u32);
    };
}

define_id!(ProgramId);
define_id!(ModuleId);
define_id!(ItemId);
define_id!(BodyId);
define_id!(ExprId);
define_id!(PatternId);
define_id!(LocalId);
define_id!(TypeId);
define_id!(TraitId);
define_id!(ImplId);
define_id!(InstanceId);
