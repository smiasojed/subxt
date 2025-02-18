// Copyright 2019-2023 Parity Technologies (UK) Ltd.
// This file is dual-licensed as Apache-2.0 or GPL-3.0.
// see LICENSE for license details.

use frame_metadata::v15::PalletMetadata;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use scale_info::form::PortableForm;

use crate::types::TypeGenerator;

use super::CodegenError;

/// Generate error type alias from the provided pallet metadata.
pub fn generate_error_type_alias(
    type_gen: &TypeGenerator,
    pallet: &PalletMetadata<PortableForm>,
    should_gen_docs: bool,
) -> Result<TokenStream2, CodegenError> {
    let Some(error) = &pallet.error else {
        return Ok(quote!());
    };

    let error_type = type_gen.resolve_type_path(error.ty.id);
    let error_ty = type_gen.resolve_type(error.ty.id);
    let docs = &error_ty.docs;
    let docs = should_gen_docs
        .then_some(quote! { #( #[doc = #docs ] )* })
        .unwrap_or_default();
    Ok(quote! {
        #docs
        pub type Error = #error_type;
    })
}
