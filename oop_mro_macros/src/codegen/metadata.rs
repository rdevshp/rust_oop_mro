use super::*;

pub(super) fn generate_metadata_impl(
    graph: &Graph,
    index: usize,
    class: &ClassDef,
) -> TokenStream2 {
    let name_ident = &class.name;
    let (impl_generics, ty_generics, where_clause) = class.generics.split_for_impl();
    let name = &graph.names[index];
    let mro = graph.mros[index].iter().map(|&mro_index| {
        let mro_name = &graph.names[mro_index];
        quote! { #mro_name }
    });
    let methods = graph.selected_methods[index].values().map(|method| {
        let method_name = method.name.to_string();
        let owner_name = &graph.names[method.owner];
        let signature = &method.signature_display;
        quote! {
            ::oop_mro::MethodEntry {
                name: #method_name,
                owner: #owner_name,
                signature: #signature,
            }
        }
    });
    let abstract_methods = graph.abstract_methods[index].values().map(|method| {
        let method_name = method.name.to_string();
        let owner_name = &graph.names[method.owner];
        let signature = &method.signature_display;
        quote! {
            ::oop_mro::MethodEntry {
                name: #method_name,
                owner: #owner_name,
                signature: #signature,
            }
        }
    });
    let is_abstract = class.is_abstract;

    quote! {
        impl #impl_generics ::oop_mro::OopClass for #name_ident #ty_generics #where_clause {
            const NAME: &'static str = #name;
            const MRO: &'static [&'static str] = &[#(#mro),*];
            const IS_ABSTRACT: bool = #is_abstract;
            const METHOD_TABLE: &'static ::oop_mro::MethodTable = &::oop_mro::MethodTable {
                methods: &[#(#methods),*],
            };
            const ABSTRACT_METHODS: &'static [::oop_mro::MethodEntry] = &[#(#abstract_methods),*];
        }

        impl #impl_generics ::oop_mro::OopObject for #name_ident #ty_generics #where_clause {
            type Class = Self;
        }
    }
}
