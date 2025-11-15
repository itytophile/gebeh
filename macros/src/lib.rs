use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{Data, DataStruct, DeriveInput, Fields, Ident, parse_macro_input};

#[proc_macro_derive(HeapSize)]
pub fn derive_heap_size(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = &input.ident;
    let name_suffixed = Ident::new(&format!("{}WriteOnce", input.ident), input.ident.span());

    let Data::Struct(DataStruct {
        fields: Fields::Named(fields),
        ..
    }) = input.data
    else {
        unreachable!()
    };

    let recurse = fields.named.iter().map(|f| {
        let name = &f.ident;
        let ty = &f.ty;
        quote_spanned! {f.span()=>
            #name: my_lib::WriteOnce<'a, #ty>,
        }
    });

    let recurse_0 = fields.named.iter().map(|f| {
        let name = &f.ident;
        quote_spanned! {f.span()=>
            #name: my_lib::WriteOnce::new(&mut self.#name),
        }
    });

    let expanded = quote! {
        struct #name_suffixed<'a> {
            #(#recurse)*
        }

        impl #name {
            fn write_once(&mut self) -> #name_suffixed {
                #name_suffixed {
                    #(#recurse_0)*
                }
            }
        }
    };

    proc_macro::TokenStream::from(expanded)
}
