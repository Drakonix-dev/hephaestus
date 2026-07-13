use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Data, DeriveInput, Fields, FnArg, Token, TraitItemFn, Type, braced,
    parse::{Parse, ParseStream},
    parse_macro_input,
};

#[proc_macro_derive(WrapFrom)]
pub fn derive_wrap_from(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let inner = match inner_type(&input) {
        Ok(ty) => ty,
        Err(e) => return e.to_compile_error().into(),
    };

    quote! {
        impl From<#inner> for #name {
            fn from(v: #inner) -> Self {
                Self(v)
            }
        }

        impl From<#name> for #inner {
            fn from(v: #name) -> Self {
                v.0
            }
        }
    }
    .into()
}

fn inner_type(input: &DeriveInput) -> syn::Result<&syn::Type> {
    let Data::Struct(data) = &input.data else {
        return Err(syn::Error::new_spanned(
            input,
            "WrapFrom only supports structs",
        ));
    };

    let Fields::Unnamed(fields) = &data.fields else {
        return Err(syn::Error::new_spanned(
            &data.fields,
            "WrapFrom requires a tuple struct",
        ));
    };

    if fields.unnamed.len() != 1 {
        return Err(syn::Error::new_spanned(
            fields,
            "WrapFrom expects exactly one field",
        ));
    }

    Ok(&fields.unnamed[0].ty)
}

struct WrapFnsInput {
    fns: Vec<TraitItemFn>,
    inner: Type,
    wrapper: Type,
}

impl Parse for WrapFnsInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let wrapper: Type = input.parse()?;
        input.parse::<Token![,]>()?;

        let inner: Type = input.parse()?;
        input.parse::<Token![,]>()?;

        let content;
        braced!(content in input);

        let mut fns = Vec::new();
        while !content.is_empty() {
            fns.push(content.parse()?);
        }

        Ok(WrapFnsInput {
            fns,
            inner,
            wrapper,
        })
    }
}

#[proc_macro]
pub fn wrap_fns(input: TokenStream) -> TokenStream {
    let WrapFnsInput {
        fns,
        inner,
        wrapper,
    } = parse_macro_input!(input as WrapFnsInput);

    let methods = fns.iter().map(|f| {
        let sig = &f.sig;
        let name = &sig.ident;
        let output = &sig.output;
        let has_receiver = matches!(sig.inputs.first(), Some(FnArg::Receiver(_)));

        let params = sig.inputs.iter().filter_map(|arg| match arg {
            FnArg::Typed(pat_type) => Some(pat_type),
            FnArg::Receiver(_) => None,
        });
        let arg_names = sig.inputs.iter().filter_map(|arg| match arg {
            FnArg::Typed(pat_type) => Some(&pat_type.pat),
            FnArg::Receiver(_) => None,
        });

        let self_param = has_receiver.then(|| quote! { &self, });
        let call = if has_receiver {
            quote! { self.0.#name(#(#arg_names.into()),*) }
        } else {
            quote! { <#inner>::#name(#(#arg_names.into()),*) }
        };

        quote! {
            pub fn #name(#self_param #(#params),*) #output {
                #call.into()
            }
        }
    });

    quote! {
        impl #wrapper {
            #(#methods)*
        }
    }
    .into()
}
