use super::llvm::LlvmCodeGen;
use crate::ast::*;
use crate::error::CompilerError;
use inkwell::types::{BasicTypeEnum, StructType};

impl<'ctx> LlvmCodeGen<'ctx> {
    
    /// Builds an LLVM struct type representing the memory layout of an object.
    pub(crate) fn build_struct_type(
        &self,
        type_def: &TypeDef,
    ) -> Result<StructType<'ctx>, CompilerError> {

        // Collect field types, propagating any error 
        let field_types: Result<Vec<BasicTypeEnum<'ctx>>, CompilerError> = type_def.attributes
            .iter()
            .map(|attr| {
                let hulk_ty = attr.ty.as_ref().ok_or_else(|| CompilerError::CodegenError {
                    msg: format!("attribute '{}' has no inferred type", attr.name),
                    span: Some(attr.span),
                })?;
                self.hulk_type_to_llvm_type(hulk_ty)
            })
            .collect();
        let field_types = field_types?;

        // let struct_name = format!("{}.obj", type_def.name);
        // let struct_ty = self.context.opaque_struct_type(&struct_name);
        // struct_ty.set_body(&field_types, false);
        // Ok(struct_ty)

        let struct_type = self.context.struct_type(&field_types, false);
        return Ok(struct_type)
    }

}