use super::llvm::LlvmCodeGen;
use crate::ast::*;
use crate::error::{CompilerError, Span};
use inkwell::types::{StructType};
use inkwell::values::{PointerValue, GlobalValue, FunctionValue, BasicMetadataValueEnum, BasicValueEnum};
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

    /// Builds the complete virtual method table (vtable) for a user‑defined type.
    ///
    /// # Layout of the generated vtable
    /// ```text
    /// { i32,              // type_id – unique numeric identifier of this class
    ///   ptr,              // parent_vtable – pointer to the vtable of the parent class, or null
    ///   [N x ptr] }       // methods – array of function pointers, sorted by vtable_index
    /// ```
    ///
    /// The vtable is emitted as a global constant so that every instance of the
    /// type can reference it through its dedicated vtable pointer (field 0 of the
    /// object struct).
    pub(crate) fn generate_vtable(
        &mut self,
        type_name: &str,
        flat: &FlattenedType,
    ) -> Result<GlobalValue<'ctx>, CompilerError> {

        // Generic opaque pointer type used for parent_vtable and method slots.
        let ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());

        // Methods array – length may be zero if the class has no methods.
        let array_type = ptr_type.array_type(flat.methods.len() as u32);

        // Define the full vtable struct: { i32, ptr, [N x ptr] }
        let vtable_type = self.context.struct_type(
            &[
                self.context.i32_type().into(),  // field 0: type_id
                ptr_type.into(),                 // field 1: parent_vtable
                array_type.into(),               // field 2: methods
            ],
            false,
        );

        // Remember the vtable type so that `visit_method_call`, `check_dynamic_type`,
        // and the runtime function `hulk_instanceof` can access its fields via GEP.
        self.vtable_types.insert(type_name.to_string(), vtable_type);

        // Create the global variable that will hold the vtable.
        let vtable = self.module.add_global(
            vtable_type,
            Some(AddressSpace::default()),
            &format!("{}_vtable", type_name),
        );

        // Build the field values

        // 1. type_id – constant i32 assigned during semantic analysis.
        let type_id_val = self.context.i32_type().const_int(flat.type_id as u64, false);

        // 2. parent_vtable – pointer to the parent's vtable, or null if this is a
        //    root class (Object or a class without explicit inherits).
        let parent_vtable_ptr = if let Some(ref parent_name) = flat.parent_name {
            // The parent's vtable must already have been generated (the caller
            // must iterate in topological order).
            self.vtables.get(parent_name)
                .map(|gv| gv.as_pointer_value())
                .unwrap_or_else(|| ptr_type.const_null())
        } else {
            ptr_type.const_null()
        };

        // 3. Methods – collect function pointers, sorted by their vtable index.
        let mut sorted: Vec<_> = flat.methods.iter().collect();
        sorted.sort_by_key(|m| m.vtable_index);
        let entries: Vec<_> = sorted.iter()
            .map(|fm| {
                // Methods follow the naming convention `{TypeName}.{methodName}`.
                let func_name = format!("{}.{}", fm.defining_type, fm.method.name);
                self.method_functions[&func_name]
                    .as_global_value()
                    .as_pointer_value()
            })
            .collect();
        let methods_array = ptr_type.const_array(&entries);

        // Assemble the complete vtable initializer.
        let initializer = vtable_type.const_named_struct(&[
            type_id_val.into(),
            parent_vtable_ptr.into(),
            methods_array.into(),
        ]);

        vtable.set_initializer(&initializer);

        Ok(vtable)
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

    /// Declares the runtime type‑checking function `hulk_instanceof`.
    ///
    /// Signature: `i1 @hulk_instanceof(i8* %obj, i32 %target_id)`
    /// Returns `true` if the object pointed to by `%obj` is of the given
    /// type or a subtype thereof.
    pub(crate) fn declare_hulk_instanceof(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("hulk_instanceof") {
            return f;
        }
        let ptr_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let i32_type = self.context.i32_type();
        let bool_type = self.context.bool_type();
        let fn_type = bool_type.fn_type(&[ptr_type.into(), i32_type.into()], false);
        self.module.add_function("hulk_instanceof", fn_type, None)
    }

    /// Generates the body of `hulk_instanceof` if it hasn't been generated yet.
    ///
    /// The function walks the VTable hierarchy starting from the object's
    /// VTable, comparing the stored `type_id` against the target.  If it
    /// reaches a null parent pointer, the check fails.
    ///
    /// To avoid hard‑coding byte offsets, we create an anonymous struct type
    /// `{ i32, ptr }` that matches the common header of every VTable.  This
    /// allows us to use safe `build_struct_gep` with field indices 0 and 1
    /// instead of raw `build_gep` with byte offsets.
    pub(crate) fn generate_hulk_instanceof_body(&mut self) -> Result<(), CompilerError> {
        let func = self.declare_hulk_instanceof();
       
        if func.count_basic_blocks() > 0 {
            return Ok(());
        }

        let entry = self.context.append_basic_block(func, "entry");
        let builder = self.context.create_builder();
        builder.position_at_end(entry);

        // Parameters
        let obj_ptr = func.get_nth_param(0).unwrap().into_pointer_value();
        let target_id = func.get_nth_param(1).unwrap().into_int_value();

        let ptr_ty = self.context.i8_type().ptr_type(AddressSpace::default());
        let i32_ty = self.context.i32_type();
        let bool_ty = self.context.bool_type();

        // Build an anonymous struct type that represents the common header of
        // every VTable:  { i32 (type_id), ptr (parent_vtable) }.
        // We don't need the methods array here, so we simply ignore it.
        let header_ty = self.context.struct_type(
            &[i32_ty.into(), ptr_ty.into()],
            false,
        );

        // Load the VTable pointer from the object (field 0)
        let vtable_ptr_ptr = builder.build_alloca(ptr_ty, "vtable_curr")
            .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: None })?;
        let vtable_ptr = builder.build_load(ptr_ty, obj_ptr, "vtable")
            .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: None })?;
        builder.build_store(vtable_ptr_ptr, vtable_ptr)
            .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: None })?;

        // Blocks
        let check_block = self.context.append_basic_block(func, "check");
        let success_block = self.context.append_basic_block(func, "true");
        let next_parent_block = self.context.append_basic_block(func, "next_parent");
        let merge_block = self.context.append_basic_block(func, "merge");

        builder.build_unconditional_branch(check_block)
            .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: None })?;

        // check block
        builder.position_at_end(check_block);
        let curr_vtable = builder.build_load(ptr_ty, vtable_ptr_ptr, "curr_vtable")
            .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: None })?;

        // Access the type_id (field 0 of the header)
        let type_id_ptr = builder.build_struct_gep(
            header_ty,
            curr_vtable.into_pointer_value(),
            0,
            "type_id_ptr",
        ).map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: None })?;
        let curr_type_id = builder.build_load(i32_ty, type_id_ptr, "curr_type_id")
            .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: None })?;
        let is_equal = builder.build_int_compare(
            inkwell::IntPredicate::EQ,
            curr_type_id.into_int_value(),
            target_id,
            "is_eq",
        ).map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: None })?;
        builder.build_conditional_branch(is_equal, success_block, next_parent_block)
            .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: None })?;

        // next_parent block
        builder.position_at_end(next_parent_block);
        // Access the parent_vtable (field 1 of the header)
        let parent_slot_ptr = builder.build_struct_gep(
            header_ty,
            curr_vtable.into_pointer_value(),
            1,
            "parent_slot",
        ).map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: None })?;
        let parent_vtable = builder.build_load(ptr_ty, parent_slot_ptr, "parent_vtable")
            .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: None })?;
        let is_null = builder.build_is_null(parent_vtable.into_pointer_value(), "is_null")
            .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: None })?;

        let update_block = self.context.append_basic_block(func, "update");
        builder.build_conditional_branch(is_null, merge_block, update_block)
            .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: None })?;

        builder.position_at_end(update_block);
        builder.build_store(vtable_ptr_ptr, parent_vtable)
            .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: None })?;
        builder.build_unconditional_branch(check_block)
            .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: None })?;

        // success block
        builder.position_at_end(success_block);
        builder.build_unconditional_branch(merge_block)
            .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: None })?;

        // merge block
        builder.position_at_end(merge_block);
        let phi = builder.build_phi(bool_ty, "result")
            .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: None })?;
        let true_val = bool_ty.const_int(1, false);
        let false_val = bool_ty.const_int(0, false);
        phi.add_incoming(&[(&true_val, success_block), (&false_val, next_parent_block)]);
        builder.build_return(Some(&phi.as_basic_value()))
            .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: None })?;

        Ok(())
    }

    //HELPERS

    /// Allocates heap memory for the given struct type, casts the raw pointer
    /// to the concrete struct pointer, and returns it.
    pub(crate) fn malloc_and_cast(
        &self,
        struct_ty: StructType<'ctx>,
        name: &str,
        span: Span,
    ) -> Result<PointerValue<'ctx>, CompilerError> {
        let size = struct_ty.size_of().ok_or_else(|| CompilerError::CodegenError {
            msg: format!("unable to determine size of type '{}'", name),
            span: Some(span),
        })?;

        // Allocate raw memory on the heap via `malloc`.
        let malloc_fn = self.declare_malloc();
        let obj_ptr = self.builder.build_call(malloc_fn, &[size.into()], "obj")
            .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: Some(span) })?
            .try_as_basic_value().left()
            .and_then(|v| v.into_pointer_value().into())
            .ok_or_else(|| CompilerError::CodegenError {
                msg: "malloc did not return a pointer".to_string(),
                span: Some(span),
            })?;

        // Cast the raw `i8*` to the concrete struct pointer.
        let typed_ptr = self.builder.build_pointer_cast(
            obj_ptr,
            struct_ty.ptr_type(AddressSpace::default()),
            &format!("{}_typed", name),
        ).map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: Some(span) })?;

        Ok(typed_ptr)
    }

    /// Stores the vtable pointer into field 0 of the given object.
    pub(crate) fn set_vtable(
        &self,
        struct_ty: StructType<'ctx>,
        obj_ptr: PointerValue<'ctx>,
        type_name: &str,
        span: Span,
    ) -> Result<(), CompilerError> {
        // Look up the vtable global for this type
        let vtable_global = *self.vtables.get(type_name).ok_or_else(|| {
            CompilerError::CodegenError {
                msg: format!("vtable not found for type '{}'", type_name),
                span: Some(span),
            }
        })?;

        // Cast the vtable pointer to opaque i8* (field 0 expects this)
        let opaque_ptr_ty = self.context.i8_type().ptr_type(AddressSpace::default());
        let vtable_ptr = self.builder.build_pointer_cast(
            vtable_global.as_pointer_value(),
            opaque_ptr_ty,
            "vtable_opaque",
        ).map_err(|e| CompilerError::CodegenError {
            msg: e.to_string(),
            span: Some(span),
        })?;

        // Store the cast vtable pointer into field 0 using the reusable helper
        self.store_field(struct_ty, obj_ptr, 0, vtable_ptr.into(), "vtable", span)
    }

    /// Stores a value into a field of the struct at the given index.
    pub(crate) fn store_field(
        &self,
        struct_ty: StructType<'ctx>,
        obj_ptr: PointerValue<'ctx>,
        field_index: u32,
        value: BasicValueEnum<'ctx>,
        field_name: &str,
        span: Span,
    ) -> Result<(), CompilerError> {
        let field_ptr = self.builder.build_struct_gep(struct_ty, obj_ptr, field_index, field_name)
            .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: Some(span) })?;
        self.builder.build_store(field_ptr, value)
            .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: Some(span) })?;
        Ok(())
    }

    /// Performs a virtual method call on the given object using its vtable.
    ///
    /// # Arguments
    /// - `obj_ptr` – pointer to the object (already cast to its concrete struct type)
    /// - `type_name` – name of the static type of the receiver (used to locate
    ///   the vtable and flattened type)
    /// - `method_name` – name of the method to call
    /// - `extra_args` – additional arguments beyond the implicit `self`, already
    ///   evaluated as LLVM metadata values
    /// - `span` – source location for error reporting
    
    pub(crate) fn call_virtual_method(
        &self,
        obj_ptr: PointerValue<'ctx>,
        type_name: &str,
        method_name: &str,
        extra_args: &[BasicMetadataValueEnum<'ctx>],
        span: Span,
    ) -> Result<(BasicValueEnum<'ctx>, HulkType), CompilerError>  {
        // Obtain the struct type of the receiver
        let struct_ty = *self.type_structs.get(type_name).ok_or_else(|| {
            CompilerError::CodegenError {
                msg: format!("unknown type '{}'", type_name),
                span: Some(span),
            }
        })?;

        // Look up the flattened type and find the method descriptor
        let flat = self.flattened_types.get(type_name).unwrap();
        let fm = flat.methods.iter()
            .find(|m| m.method.name == method_name)
            .ok_or_else(|| CompilerError::CodegenError {
                msg: format!("method '{}' not found in type '{}'", method_name, type_name),
                span: Some(span),
            })?;

        // Load the vtable pointer from field 0 of the object
        let opaque_ptr_ty = self.context.i8_type().ptr_type(AddressSpace::default());
        let vtable_slot = self.builder.build_struct_gep(struct_ty, obj_ptr, 0, "vtable_slot")
            .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: Some(span) })?;
        let vtable_ptr = self.builder.build_load(opaque_ptr_ty, vtable_slot, "vtable")
            .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: Some(span) })?
            .into_pointer_value();

        // Access the methods array (field 2 of the vtable)
        let vtable_ty = *self.vtable_types.get(type_name).unwrap();
        let methods_ptr = self.builder.build_struct_gep(vtable_ty, vtable_ptr, 2, "methods")
            .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: Some(span) })?;

        // Index into the methods array using the method's vtable index
        let method_array_ty = opaque_ptr_ty.array_type(flat.methods.len() as u32);
        let method_slot = unsafe {
            self.builder.build_gep(
                method_array_ty,
                methods_ptr,
                &[
                    self.context.i32_type().const_zero(),
                    self.context.i32_type().const_int(fm.vtable_index as u64, false),
                ],
                "method_slot",
            )
        }.map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: Some(span) })?;

        let raw_fn = self.builder.build_load(opaque_ptr_ty, method_slot, "method")
            .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: Some(span) })?
            .into_pointer_value();

        // Retrieve the LLVM function signature using the defining type
        let func_name = format!("{}.{}", fm.defining_type, method_name);
        let declared = *self.method_functions.get(&func_name).ok_or_else(|| {
            CompilerError::CodegenError {
                msg: format!("method '{}' not declared", func_name),
                span: Some(span),
            }
        })?;
        let fn_type = declared.get_type();

        // Build the argument list: self + extra arguments
        let mut call_args: Vec<BasicMetadataValueEnum<'ctx>> = vec![obj_ptr.into()];
        call_args.extend_from_slice(extra_args);

        // Emit the indirect call
        let call = self.builder.build_indirect_call(fn_type, raw_fn, &call_args, "calltmp")
            .map_err(|e| CompilerError::CodegenError { msg: e.to_string(), span: Some(span) })?;

        let ret_hulk = fm.method.ty.clone()
            .unwrap_or(HulkType::Object); 

        let result_value = call.try_as_basic_value().left().unwrap().into();
            Ok((result_value, ret_hulk))
    }

}