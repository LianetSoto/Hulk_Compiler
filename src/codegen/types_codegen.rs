use super::llvm::LlvmCodeGen;
use crate::ast::*;
use crate::error::CompilerError;
use inkwell::types::{StructType};
use inkwell::values::{PointerValue, GlobalValue};
use inkwell::AddressSpace;
use crate::semantic::{HulkType, FlattenedType};

impl<'ctx> LlvmCodeGen<'ctx> {
    
    /// Builds the LLVM struct type for a class using its flattened attributes.
    pub(crate) fn build_struct_type_from_flat(
        &self,
        flat: &FlattenedType,
    ) -> Result<StructType<'ctx>, CompilerError> {
        let mut field_types = Vec::new();

        // VTable pointer – stored as opaque i8*
        let opaque_ptr = self.context.i8_type().ptr_type(AddressSpace::default());
        field_types.push(opaque_ptr.into());

        // Attributes from flattened list (already ordered: parent → child)
        for attr in &flat.attributes {
            let hulk_ty = attr.ty.as_ref().ok_or_else(|| CompilerError::CodegenError {
                msg: format!("type not inferred for attribute '{}'", attr.name),
                span: Some(attr.span),
            })?;
            let llvm_ty = self.hulk_type_to_llvm_type(hulk_ty)?;
            field_types.push(llvm_ty);
        }

        let struct_type = self.context.struct_type(&field_types, false);
        Ok(struct_type)
    }

    /// Generates the virtual method table (vtable) for a user‑defined type.
    ///
    /// In HULK every user‑defined type that contains at least one method receives
    /// a **vtable**: a global constant array of function pointers.  This table is
    /// stored as an LLVM global variable and is referenced by every instance of
    /// the type through a dedicated pointer (the first field of the struct).
    pub(crate) fn generate_vtable(
        &self,
        type_name: &str,
        flat: &FlattenedType,
    ) -> Result<Option<GlobalValue<'ctx>>, CompilerError> {

        // If the type has no methods there is nothing to dispatch → no vtable.
        if flat.methods.is_empty() {
            return Ok(None);
        }

        // [N x ptr]
        let ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let array_type = ptr_type.array_type(flat.methods.len() as u32);

        // Create a global variable that will hold the vtable.
        let vtable = self.module.add_global(
            array_type,
            Some(AddressSpace::default()),
            &format!("{}_vtable", type_name),
        );

        // Sort methods by their vtable index – the order must be stable and
        // match the indices that `visit_method_call` uses for indirect dispatch.
        let mut sorted: Vec<_> = flat.methods.iter().collect();
        sorted.sort_by_key(|m| m.vtable_index);

        // Build the array of function pointers.
        let mut entries = Vec::with_capacity(sorted.len());

        for fm in sorted {

            // Methods are registered with the name convention `{Type}.{method}`
            // (e.g. `Point.getX`).  The flattened method records the *defining*
            // type, which is the class that introduced the implementation.
            let func_name = format!("{}.{}", fm.defining_type, fm.method.name);
 
            let func = self.method_functions.get(&func_name).ok_or_else(|| {
                CompilerError::CodegenError {
                    msg: format!("vtable: method '{}' not found", func_name),
                    span: None,
                }
            })?;

            // Store the function as a generic `i8*` pointer.
            entries.push(func.as_global_value().as_pointer_value());
        }

        // Create a constant array initialiser and attach it to the global.
        let initializer = ptr_type.const_array(&entries);
        vtable.set_initializer(&initializer);

        Ok(Some(vtable))
    }

    /// Computes a pointer to the struct field described by an attribute-access expression.
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
                    msg: format!(
                        "cannot access attribute of non-object type {:?}",
                        obj_hulk_ty
                    ),
                    span: Some(attr.span),
                });
            }
        };

        // LLVM struct type.
        let struct_ty = *self.type_structs.get(&type_name).ok_or_else(|| {
            CompilerError::CodegenError {
                msg: format!("unknown type '{}'", type_name),
                span: Some(attr.span),
            }
        })?;

        // Flattened type (contains inherited attributes in layout order).
        let flat = self.flattened_types.get(&type_name).ok_or_else(|| {
            CompilerError::CodegenError {
                msg: format!("flattened type '{}' not found", type_name),
                span: Some(attr.span),
            }
        })?;

        // Find the attribute in the flattened layout.
        let attr_index = flat.attributes
            .iter()
            .position(|a| a.name == attr.attribute)
            .ok_or_else(|| CompilerError::CodegenError {
                msg: format!(
                    "attribute '{}' not found in type '{}'",
                    attr.attribute,
                    type_name
                ),
                span: Some(attr.span),
            })?;

        // Field 0 is the vtable pointer, so attributes start at index 1.
        let field_ptr = self.builder
            .build_struct_gep(
                struct_ty,
                obj_ptr,
                (attr_index + 1) as u32,
                &format!("field_{}", attr.attribute),
            )
            .map_err(|e| CompilerError::CodegenError {
                msg: format!(
                    "GEP for attribute '{}' failed: {}",
                    attr.attribute,
                    e
                ),
                span: Some(attr.span),
            })?;

        // Retrieve the attribute type.
        let attr_hulk_ty = flat.attributes[attr_index]
            .ty
            .as_ref()
            .ok_or_else(|| CompilerError::CodegenError {
                msg: format!(
                    "type not inferred for attribute '{}'",
                    attr.attribute
                ),
                span: Some(attr.span),
            })?
            .clone();

        Ok((field_ptr, attr_hulk_ty))
    }

}