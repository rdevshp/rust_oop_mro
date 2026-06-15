use super::*;

pub(super) fn generate_static_fields(graph: &Graph) -> TokenStream2 {
    let statics = graph.classes.iter().enumerate().flat_map(|(index, class)| {
        class.items.iter().filter_map(move |item| match item {
            ClassItem::StaticField(static_field) => {
                Some(generate_static_field(graph, index, static_field))
            }
            _ => None,
        })
    });

    quote! {
        #(#statics)*
    }
}

fn generate_static_field(
    graph: &Graph,
    class_index: usize,
    static_field: &StaticFieldDef,
) -> TokenStream2 {
    let attrs = &static_field.attrs;
    let static_token = &static_field.static_token;
    let ident = static_field_ident(&graph.names[class_index], &static_field.ident.to_string());
    let colon = &static_field.colon_token;
    let ty = &static_field.ty;
    let eq = &static_field.eq_token;
    let expr = &static_field.expr;
    let semi = &static_field.semi_token;

    quote! {
        #(#attrs)*
        #static_token #ident #colon #ty #eq #expr #semi
    }
}
