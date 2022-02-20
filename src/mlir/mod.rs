pub mod raw {
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]
    #![allow(dead_code)]

    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

pub mod attribute;
pub mod block;
pub mod operation;
pub mod region;
pub mod types;
pub mod value;

use std::{marker::PhantomData, mem::ManuallyDrop};

use operation::Operation;
use raw::*;

impl From<&str> for MlirStringRef {
    fn from(item: &str) -> Self {
        MlirStringRef {
            data: item.as_ptr() as *const _,
            length: item.len() as u64,
        }
    }
}

/// Wrapper around the C API's MlirContext.
pub struct Context {
    inner: MlirContext,
}

impl Context {
    /// Make an empty MLIR context.
    ///
    /// Currently, this also registers all dialects and all passes for your convenience;
    /// which is not particularly efficient and is subject to change.
    pub fn new() -> Self {
        unsafe {
            let inner = mlirContextCreate();
            mlirRegisterAllDialects(inner);
            mlirRegisterAllPasses();
            Context { inner }
        }
    }

    /// Make a source location from a `filename`, a `line` number and a `column` number.
    ///
    /// The object is created in, and owned by the context.
    pub fn get_location(&self, filename: &str, line: usize, column: usize) -> Location<'_> {
        Location {
            inner: unsafe {
                mlirLocationFileLineColGet(
                    self.as_raw(),
                    filename.into(),
                    line as u32,
                    column as u32,
                )
            },
            _marker: PhantomData,
        }
    }

    /// Make an unknown source location.
    ///
    /// The object is created in, and owned by the context.
    pub fn get_unknown_location(&self) -> Location<'_> {
        Location {
            inner: unsafe { mlirLocationUnknownGet(self.as_raw()) },
            _marker: PhantomData,
        }
    }

    /// Return the underlying raw MlirAttribute.
    pub fn as_raw(&self) -> MlirContext {
        self.inner
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe { mlirContextDestroy(self.inner) }
    }
}

/// Wrapper around the C API's MlirModule.
pub struct Module {
    inner: MlirModule,
}

impl Module {
    /// Make an empty MLIR Module from a source location.
    pub fn new(location: Location<'_>) -> Self {
        Module {
            inner: unsafe { mlirModuleCreateEmpty(location.into_raw()) },
        }
    }

    /// Append an `operation` to the module's only body block.
    ///
    /// We make the opinionated choice of only exposing the block
    /// this way for now.
    pub fn append(&mut self, operation: Operation) {
        unsafe {
            mlirBlockAppendOwnedOperation(mlirModuleGetBody(self.inner), operation.into_raw())
        }
    }

    /// Return the underlying raw MlirModule.
    pub fn as_raw(&self) -> MlirModule {
        self.inner
    }

    /// Return the underlying raw MlirModule and consume the Module.
    pub fn into_raw(self) -> MlirModule {
        ManuallyDrop::new(self).inner
    }
}

impl Drop for Module {
    fn drop(&mut self) {
        unsafe { mlirModuleDestroy(self.inner) }
    }
}

#[derive(Clone, Copy)]
/// Wrapper around the C API's MlirLocation.
pub struct Location<'l> {
    /// Opaque pointer the data across the FFI, generally a C++ object.
    inner: MlirLocation,
    /// Force the type to "own" a reference to the context it was created in,
    /// so that its lifetime may be the same as that of the context.
    _marker: PhantomData<&'l ()>,
}

impl Location<'_> {
    /// Unwrap the Location, returning the underlying MlirLocation.
    fn into_raw(self) -> MlirLocation {
        self.inner
    }
}

/// Wrapper around the C API's MlirPassManager.
pub struct Pass {
    pass: MlirPassManager,
}

impl Pass {
    /// Make an empty MLIR pass.
    ///
    /// See the dialect_to_dialect() methods for available conversions.
    pub fn new(context: &Context) -> Self {
        Pass {
            pass: unsafe { mlirPassManagerCreate(context.as_raw()) },
        }
    }

    /// Standard to LLVM conversion pass.
    pub fn std_to_llvm(self) -> Self {
        unsafe {
            let conversion = mlirCreateConversionConvertStandardToLLVM();
            mlirPassManagerAddOwnedPass(self.pass, conversion);
        }
        self
    }

    /// SCF to OpenMP conversion pass.
    pub fn scf_to_openmp(self) -> Self {
        unsafe {
            let conversion = mlirCreateConversionConvertSCFToOpenMP();
            mlirPassManagerAddOwnedPass(self.pass, conversion);
        }
        self
    }

    /// OpenMP to LLVM conversion pass.
    pub fn openmp_to_llvm(self) -> Self {
        unsafe {
            let conversion = mlirCreateConversionConvertOpenMPToLLVM();
            mlirPassManagerAddOwnedPass(self.pass, conversion);
        }
        self
    }

    /// Run the pass on a specified module.
    ///
    /// Doesn't consume the pass so you can reuse it on other multiple modules.
    pub fn run(&self, module: &mut Module) {
        // TODO: Do proper error handling with the LogicalResult.
        unsafe {
            mlirPassManagerRun(self.pass, module.as_raw());
        }
    }
}

impl Drop for Pass {
    fn drop(&mut self) {
        unsafe { mlirPassManagerDestroy(self.pass) }
    }
}
