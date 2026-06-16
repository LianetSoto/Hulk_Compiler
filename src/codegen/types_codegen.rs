use super::llvm::LlvmCodeGen;
use crate::ast::*;
use crate::semantic::HulkType;
use crate::error::CompilerError;
use inkwell::types::{BasicTypeEnum, StructType};
use inkwell::values::{PointerValue};

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

    /// Computes a pointer to the struct field described by an attribute‑access expression.
    /// Returns the LLVM pointer and the HULK type of the attribute.
    pub(crate) fn get_attribute_ptr(
        &mut self,
        attr: &mut AttributeAccessExpr,
    ) -> Result<(PointerValue<'ctx>, HulkType), CompilerError> {

        // Evaluate the object expression → obtain a pointer to the struct.
        let obj_val = attr.object.accept(self)?;
        let obj_ptr = obj_val.into_pointer_value();

        // Retrieve the HULK type of the object (must be a Class).
        let obj_hulk_ty = attr.object.get_type().ok_or_else(|| {
            CompilerError::CodegenError {
                msg: "type not inferred for object in attribute access".to_string(),
                span: Some(attr.span),
            }
        })?;

        let type_name = match obj_hulk_ty {
            HulkType::Class(name) => name.clone(),
            _ => {
                return Err(CompilerError::CodegenError {
                    msg: format!("cannot access attribute of non‑object type {:?}", obj_hulk_ty),
                    span: Some(attr.span),
                });
            }
        };

        // Obtain the LLVM struct type and the AST definition of the class.
        let struct_ty = self.type_structs.get(&type_name).ok_or_else(|| {
            CompilerError::CodegenError {
                msg: format!("unknown type '{}'", type_name),
                span: Some(attr.span),
            }
        })?;

        let type_def = self.type_defs.get(&type_name).ok_or_else(|| {
            CompilerError::CodegenError {
                msg: format!("type definition not found for '{}'", type_name),
                span: Some(attr.span),
            }
        })?;

        // Find the index of the attribute in the struct layout.
        let attr_index = type_def.attributes.iter()
            .position(|a| a.name == attr.attribute)
            .ok_or_else(|| CompilerError::CodegenError {
                msg: format!(
                    "attribute '{}' not found in type '{}'",
                    attr.attribute, type_name
                ),
                span: Some(attr.span),
            })?;

        // Compute the address of the field using GEP.
        let field_ptr = self.builder.build_struct_gep(
            *struct_ty,
            obj_ptr,
            attr_index as u32,
            &format!("field_{}", attr_index),
        )
        .map_err(|e| CompilerError::CodegenError {
            msg: format!("GEP for attribute '{}' failed: {}", attr.attribute, e),
            span: Some(attr.span),
        })?;

        // Retrieve the HULK type of the attribute (already inferred by TypeChecker).
        let attr_hulk_ty = type_def.attributes[attr_index]
            .ty
            .as_ref()
            .ok_or_else(|| CompilerError::CodegenError {
                msg: format!("type not inferred for attribute '{}'", attr.attribute),
                span: Some(attr.span),
            })?
            .clone();

        Ok((field_ptr, attr_hulk_ty))
    }

}