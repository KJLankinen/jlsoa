/*
 * Necessary derive macros:
 * - create a trait and implement it (hould the trait be created? That also hides the
 * implementation, similar to types
 * -
 * - should these things be implemented inside a mod? Something like
 *   pub mod soa { enum Index, enum Field, struct Radius, struct Position, struct Tag }
 * - implement aos trait
 *
 * */

use quote::{format_ident, quote, quote_spanned};
use syn::{
    parse_macro_input, spanned::Spanned, Data, DeriveInput, Fields, FieldsNamed, FieldsUnnamed,
    Ident, Index,
};

#[proc_macro_derive(Aos)]
pub fn derive_aos(token_stream: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(token_stream as DeriveInput);
    let name = format_ident!("{}", input.ident);
    let trait_name = format_ident!("{}Accessor", input.ident);

    let num_fields = generator(
        &input.data,
        |fields| fields.named.len(),
        |fields| fields.unnamed.len(),
    );

    let layout_arms = generator(
        &input.data,
        |fields| {
            let recurse = fields.named.iter().enumerate().map(|(i, f)| {
                let index = Index::from(i);
                let field_type = &f.ty;
                quote_spanned! {f.span() =>
                    #index => std::alloc::Layout::new::<#field_type>(),
                }
            });
            quote! {
                #(#recurse)*
            }
        },
        |fields| {
            let recurse = fields.unnamed.iter().enumerate().map(|(i, f)| {
                let index = Index::from(i);
                let field_type = &f.ty;
                quote_spanned! {f.span() =>
                    #index => std::alloc::Layout::new::<#field_type>(),
                }
            });
            quote! {
                #(#recurse)*
            }
        },
    );

    let offset_arms = generator(
        &input.data,
        |fields| {
            let recurse = fields.named.iter().enumerate().map(|(i, f)| {
                let index = Index::from(i);
                let name = &f.ident;
                quote_spanned! {f.span() =>
                    #index => std::mem::offset_of!(Self, #name),
                }
            });
            quote! {
                #(#recurse)*
            }
        },
        |fields| {
            let recurse = fields.unnamed.iter().enumerate().map(|(i, f)| {
                let index = Index::from(i);
                quote_spanned! {f.span() =>
                    #index => std::mem::offset_of!(Self, #index),
                }
            });
            quote! {
                #(#recurse)*
            }
        },
    );

    let trait_fn_declarations = generator(
        &input.data,
        |fields| {
            let recurse = fields.named.iter().map(|f| {
                let name = &f.ident;
                let name_mut =
                    Ident::new(&format!("{}_mut", name.as_ref().unwrap()), f.ident.span());
                let field_type = &f.ty;
                quote_spanned! {f.span() =>
                    fn #name(&self) -> &'p[#field_type];
                    fn #name_mut(&mut self) -> &'p mut [#field_type];
                }
            });
            quote! {
                #(#recurse)*
            }
        },
        |fields| {
            let recurse = fields.unnamed.iter().enumerate().map(|(i, f)| {
                let index = Index::from(i);
                let name = format_ident!("field{}", index);
                let name_mut = format_ident!("field{}_mut", index);
                let field_type = &f.ty;
                quote_spanned! {f.span() =>
                    fn #name(&self) -> &'p[#field_type];
                    fn #name_mut(&mut self) -> &'p mut[#field_type];
                }
            });
            quote! {
                #(#recurse)*
            }
        },
    );

    let trait_fn_definitions = generator(
        &input.data,
        |fields| {
            let recurse = fields.named.iter().enumerate().map(|(i, f)| {
                let name = &f.ident;
                let name_mut =
                    Ident::new(&format!("{}_mut", name.as_ref().unwrap()), f.ident.span());
                let index = Index::from(i);
                let field_type = &f.ty;
                quote_spanned! {f.span() =>
                    fn #name(&self) -> &'p[#field_type] {
                        self.get_slice::<#field_type, #index>()
                    }

                    fn #name_mut(&mut self) -> &'p mut [#field_type] {
                        self.get_mut_slice::<#field_type, #index>()
                    }
                }
            });
            quote! {
                #(#recurse)*
            }
        },
        |fields| {
            let recurse = fields.unnamed.iter().enumerate().map(|(i, f)| {
                let index = Index::from(i);
                let name = format_ident!("field{}", index);
                let name_mut = format_ident!("field{}_mut", index);
                let field_type = &f.ty;
                quote_spanned! {f.span() =>
                    fn #name(&self) -> &'p[#field_type] {
                        self.get_slice::<#field_type, #index>()
                    }
                    fn #name_mut(&mut self) -> &'p mut[#field_type] {
                        self.get_mut_slice::<#field_type, #index>()
                    }
                }
            });
            quote! {
                #(#recurse)*
            }
        },
    );

    let expanded = quote! {
        impl StructMetadata for #name {
            const NUM_FIELDS: usize = #num_fields;

            fn layout(i: usize) -> std::alloc::Layout {
                match i {
                    #layout_arms
                    _ => panic!("Too large index"),
                }
            }

            fn offset_of(i: usize) -> usize {
                match i {
                    #offset_arms
                    _ => panic!("Too large index"),
                }
            }
        }

        impl Aos for #name {
            type RefType = ();
            type MutRefType = ();
        }

        pub trait #trait_name<'p> {
            #trait_fn_declarations
        }

        impl<'p, const N: usize> #trait_name<'p> for Soa<'p, #name, { #name::NUM_FIELDS }, N> {
            #trait_fn_definitions
        }
    };

    // Too see the output as a compiler error, uncomment this
    //panic!("{}", proc_macro::TokenStream::from(expanded).to_string());
    proc_macro::TokenStream::from(expanded)
}

fn generator<T>(
    data: &Data,
    named: impl Fn(&FieldsNamed) -> T,
    unnamed: impl Fn(&FieldsUnnamed) -> T,
) -> T {
    match *data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => named(fields),
            Fields::Unnamed(ref fields) => unnamed(fields),
            Fields::Unit => unimplemented!(),
        },
        // This is not supposed to be used on enums or unions, only structs
        Data::Enum(_) | Data::Union(_) => unimplemented!(),
    }
}
