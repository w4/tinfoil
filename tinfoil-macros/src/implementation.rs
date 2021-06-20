use darling::FromField;
use quote::quote;
use syn::{spanned::Spanned, Data, DeriveInput, Type};

type Result<T> = std::result::Result<T, proc_macro::TokenStream>;

fn replace_lifetimes_with_static(ty: &Type) -> Type {
    match ty {
        syn::Type::Reference(v) => {
            let mut v = v.clone();
            v.lifetime = Some(syn::Lifetime::new("'static", v.span()));
            if let syn::Type::Path(path) = v.elem.as_mut() {
                for segment in &mut path.path.segments {
                    if let syn::PathArguments::AngleBracketed(bracketed) = &mut segment.arguments {
                        for arg in &mut bracketed.args {
                            if let syn::GenericArgument::Lifetime(lifetime) = arg {
                                *lifetime = syn::Lifetime::new("'static", ty.span());
                            }
                        }
                    }
                }
            }
            syn::Type::Reference(v)
        }
        v => v.clone(),
    }
}

pub fn tinfoil(input: proc_macro::TokenStream) -> Result<proc_macro::TokenStream> {
    let input: DeriveInput = syn::parse(input).map_err(|e| e.to_compile_error())?;

    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let s = match input.data {
        Data::Struct(v) => v,
        _ => panic!("not a struct"),
    };
    let types = s
        .fields
        .iter()
        .map(|v| replace_lifetimes_with_static(&v.ty));
    let fields = s.fields.iter().map(|v| &v.ident);

    let expanded = quote! {
        #[allow(missing_docs, single_use_lifetimes)]
        impl #impl_generics Dependency<'a, InjectionContext<'a>> for #name #ty_generics #where_clause {
            const DEPENDENCIES: &'static [std::any::TypeId] = &[
                #(std::any::TypeId::of::<#types>()),*
            ];

            fn instn(context: &'a InjectionContext<'a>) -> Self {
                Self {
                    #(#fields: context.get()),*
                }
            }
        }
    };

    Ok(expanded.into())
}

#[derive(Clone, Debug, FromField)]
#[darling(attributes(tinfoil))]
struct TinfoilContextFieldOpts {
    #[darling(default)]
    parameter: bool,
    #[darling(default)]
    default: bool,
}

pub fn tinfoil_context(input: proc_macro::TokenStream) -> Result<proc_macro::TokenStream> {
    let input: DeriveInput = syn::parse(input).map_err(|e| e.to_compile_error())?;

    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let s = match input.data {
        Data::Struct(ref v) => v,
        _ => panic!("not a struct"),
    };

    let mut parameters = Vec::new();
    let mut parameter_types = Vec::new();

    let mut defaults = Vec::new();
    let mut defaults_types = Vec::new();

    let mut instantiate_from_ctx = Vec::new();
    let mut instantiate_from_ctx_types = Vec::new();

    for field in s.fields.iter() {
        let ident = field.ident.clone().unwrap();
        let field_opts = TinfoilContextFieldOpts::from_field(field).unwrap();

        if field_opts.parameter {
            parameters.push(ident);
            parameter_types.push(field.ty.clone());
        } else if field_opts.default {
            defaults.push(ident);
            defaults_types.push(field.ty.clone());
        } else if ident != "_pin" {
            match &field.ty {
                syn::Type::Path(v) => {
                    let segment = v.path.segments.first().unwrap();
                    if segment.ident != "MaybeUninit" {
                        return Err(syn::Error::new(
                            v.span(),
                            "values instantiated via a context must be wrapped in MaybeUninit",
                        )
                        .into_compile_error()
                        .into());
                    }
                    if let syn::PathArguments::AngleBracketed(
                        syn::AngleBracketedGenericArguments { args, .. },
                    ) = &segment.arguments
                    {
                        match args.first().unwrap() {
                            syn::GenericArgument::Type(syn::Type::Path(t)) => {
                                instantiate_from_ctx_types
                                    .push(t.path.segments.first().unwrap().clone().ident);
                            }
                            _ => panic!("invalid type"),
                        }
                    }
                }
                v => {
                    return Err(syn::Error::new(v.span(), "value must be owned")
                        .into_compile_error()
                        .into())
                }
            }

            instantiate_from_ctx.push(ident);
        }
    }

    let expanded = quote! {
        impl #impl_generics #name #ty_generics #where_clause {
            pub fn new(#(#parameters: #parameter_types),*) -> Pin<Box<Self>> {
                let mut context = Box::pin(InjectionContext {
                    #(#parameters,)*
                    #(#defaults: Default::default(),)*
                    #(#instantiate_from_ctx: MaybeUninit::uninit(),)*
                    _pin: PhantomPinned,
                });

                let mut dag: tinfoil::internals::Dag<i32, i32, usize> = tinfoil::internals::Dag::new();
                let initial = dag.add_node(0);
                let mut nodes = std::collections::HashMap::new();
                // TODO: fix generics & run through `replace_lifetimes_with_static`
                #(nodes.insert(std::any::TypeId::of::<&'static #instantiate_from_ctx_types<'static>>(), {
                    let node = dag.add_node(0);
                    dag.add_edge(initial, node, 0);
                    node
                });)*

                #(println!("{:?} - {}", std::any::TypeId::of::<&'static #instantiate_from_ctx_types<'static>>(), stringify!(#instantiate_from_ctx_types));)*

                #(
                    let node = nodes.get(&std::any::TypeId::of::<&'static #instantiate_from_ctx_types<'static>>()).unwrap();

                    for dependency in #instantiate_from_ctx_types::DEPENDENCIES {
                        eprintln!("{:#?}", nodes);
                        if let Some(dependency_node) = nodes.get(dependency) {
                            dag.add_edge(node.clone(), dependency_node.clone(), 0).expect("dependency cycle");
                        }
                    }
                )*

                eprintln!("{:?}", tinfoil::internals::petgraph::dot::Dot::with_config(dag.graph(), &[tinfoil::internals::petgraph::dot::Config::EdgeIndexLabel]));

                let mut bfs = tinfoil::internals::petgraph::visit::Dfs::new(dag.graph(), initial);
                while let Some(nx) = bfs.next(dag.graph()) {
                    // skip root
                    if nx == initial { continue; }

                    let ty = *nodes.iter().find(|(k, v)| **v == nx).expect("couldn't find index").0;

                    if false {}
                    #(else if ty == std::any::TypeId::of::<&'static #instantiate_from_ctx_types<'static>>() {
                        // this is safe because we know MyCoolValue is initialised and will be for the lifetime of
                        // InjectionContext
                        let value = MaybeUninit::new(#instantiate_from_ctx_types::instn(context.as_ref().get_ref()));

                        // this is safe because we don't move any values
                        unsafe {
                            // let mut_ref: Pin<&mut InjectionContext> = Pin::as_mut(&mut context);
                            let mut_ref: Pin<&mut InjectionContext> = std::mem::transmute(context.as_ref());
                            Pin::get_unchecked_mut(mut_ref).#instantiate_from_ctx = value;
                        }
                    })*
                    else {
                        panic!("unknown type");
                    }
                }

                context
            }
        }

        #(impl #impl_generics Provider<'a, &'a #parameter_types> for #name #ty_generics #where_clause {
            fn get(&'a self) -> &'a #parameter_types {
                &self.#parameters
            }
        })*

        #(impl #impl_generics Provider<'a, &'a #defaults_types> for #name #ty_generics #where_clause {
            fn get(&'a self) -> &'a #defaults_types {
                &self.#defaults
            }
        })*


        #(impl #impl_generics Provider<'a, &'a #instantiate_from_ctx_types<'a>> for #name #ty_generics #where_clause {
            fn get(&'a self) -> &'a #instantiate_from_ctx_types<'a> {
                unsafe { &*self.#instantiate_from_ctx.as_ptr() }
            }
        })*
    };

    Ok(expanded.into())
}
