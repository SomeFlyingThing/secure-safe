use proc_macro::TokenStream;
use syn::{Data, DeriveInput, parse_macro_input};

#[proc_macro_derive(Zip, attributes(level))]
pub fn zip(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let level = input.attrs.iter().find(|attr| attr.path().is_ident("level")).map(|attr| attr.parse_args::<syn::LitInt>().unwrap().base10_parse::<u32>().unwrap_or(5));
    let name = input.ident;

    let data = match &input.data {
        Data::Struct(data) => data,
        _ => panic!("only allowed in structs"),
    };

    let fields: Vec<_> = data.fields.iter().map(|field| field.ident.as_ref().unwrap()).collect();

    quote::quote! {


            struct Zipped{
                #(
                    #fields: Vec<u8>,
                )*
            }

            impl Zip for #name {
                fn zip(&self)-> Zipped{
    Zipped{
                    #(
                        #fields: encode_all(bincode::serialize(self.#fields).unwrap()).unwrap(),
                    )*
           }
                }
            }
        }
    .into()
}
