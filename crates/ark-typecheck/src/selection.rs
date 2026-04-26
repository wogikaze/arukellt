use ark_hir::{Selection, SelectionKind, Ty};

pub(crate) fn selection_kind_for_method_name(name: &str, is_operator: bool) -> SelectionKind {
    if name.ends_with("__from") || name.ends_with("_from") || name.ends_with("__from") {
        SelectionKind::FromConversion
    } else if is_operator {
        SelectionKind::Operator
    } else {
        SelectionKind::Method
    }
}

pub(crate) fn make_selection(
    resolved_function: String,
    self_ty: Option<Ty>,
    method_item_id: Option<ark_hir::ItemId>,
    impl_id: Option<ark_hir::ImplId>,
    kind: SelectionKind,
) -> Selection {
    Selection {
        kind,
        impl_id,
        method_item_id,
        generic_substitutions: vec![],
        self_ty,
        resolved_function: resolved_function.clone(),
    }
}
