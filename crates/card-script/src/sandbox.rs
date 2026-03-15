//! Lua sandbox configuration.
//!
//! Provides a sandboxed Lua VM for card script execution.
//! This is the ONLY location in card-script where unsafe mlua FFI may be
//! triggered. All other modules must use [`create_sandboxed_lua()`] instead
//! of `Lua::new()` directly.

use mlua::{Lua, LuaOptions, StdLib};

use crate::error::ScriptError;

/// Create a sandboxed [`Lua`] VM for executing card scripts.
///
/// Allowed standard libraries: TABLE, STRING, MATH.
/// Blocked: IO, OS, DEBUG, PACKAGE, COROUTINE.
/// Memory limit: 16 MiB.
pub fn create_sandboxed_lua() -> Result<Lua, ScriptError> {
    let libs = StdLib::TABLE | StdLib::STRING | StdLib::MATH;

    let lua = Lua::new_with(libs, LuaOptions::default()).map_err(ScriptError::LuaError)?;

    lua.set_memory_limit(16 * 1024 * 1024)
        .map_err(ScriptError::LuaError)?;

    // Remove any dangerous global functions that may still be present
    {
        let globals = lua.globals();
        for name in &["loadfile", "dofile", "require", "load", "collectgarbage"] {
            // Silently ignore errors -- these may not exist in the restricted VM
            let _ = globals.raw_set(*name, mlua::Value::Nil);
        }
    }

    Ok(lua)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sandbox_creates_successfully() {
        let lua = create_sandboxed_lua().expect("sandbox creation failed");
        let result: i32 = lua.load("return 1 + 2").eval().unwrap();
        assert_eq!(result, 3);
    }

    #[test]
    fn sandbox_string_lib_available() {
        let lua = create_sandboxed_lua().unwrap();
        let result: String = lua.load("return string.upper('hello')").eval().unwrap();
        assert_eq!(result, "HELLO");
    }

    #[test]
    fn sandbox_math_lib_available() {
        let lua = create_sandboxed_lua().unwrap();
        let result: f64 = lua.load("return math.floor(3.7)").eval().unwrap();
        assert!((result - 3.0).abs() < 1e-10);
    }

    #[test]
    fn sandbox_table_lib_available() {
        let lua = create_sandboxed_lua().unwrap();
        let result: i32 = lua
            .load("local t = {10, 20, 30}; return #t")
            .eval()
            .unwrap();
        assert_eq!(result, 3);
    }

    #[test]
    fn sandbox_io_not_available() {
        let lua = create_sandboxed_lua().unwrap();
        let val: mlua::Value = lua.load("return io").eval().unwrap_or(mlua::Value::Nil);
        assert!(
            matches!(val, mlua::Value::Nil),
            "io should not be accessible"
        );
    }

    #[test]
    fn sandbox_os_not_available() {
        let lua = create_sandboxed_lua().unwrap();
        let val: mlua::Value = lua.load("return os").eval().unwrap_or(mlua::Value::Nil);
        assert!(
            matches!(val, mlua::Value::Nil),
            "os should not be accessible"
        );
    }

    #[test]
    fn sandbox_require_removed() {
        let lua = create_sandboxed_lua().unwrap();
        let result: mlua::Result<mlua::Value> = lua.load("return require").eval();
        match result {
            Ok(mlua::Value::Nil) => {}
            Ok(_) => panic!("require should not be available"),
            Err(_) => {}
        }
    }
}
