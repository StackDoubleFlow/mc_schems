use super::{Schematic, SchematicError, SchematicFormat};
use nbt::Value;

macro_rules! required_nbt {
    ($nbt:expr, $name:literal, $ty:ident) => {
        match $nbt.get($name) {
            Some(Value::$ty(value)) => value,
            Some(_) => return Err(SchematicError::MistypedField($name.to_owned())),
            None => return Err(SchematicError::MissingRequiredField($name.to_owned())),
        }
    };
}

macro_rules! typed_nbt {
    ($value:expr, $name:literal, $ty:ident) => {
        match nbt.get($name) {
            Some(Value::$ty(value)) => Some(value),
            Some(_) => return Err(SchematicError::MistypedField($name.to_owned())),
            None => None,
        }
    };
}

pub fn deserialize(nbt: &nbt::Blob, version: u32) -> Result<Schematic, SchematicError> {
    if version != 2 {
        return Err(SchematicError::UnsupportedFormat(
            SchematicFormat::Litematica(version),
        ));
    }
    let data_version = *required_nbt!(nbt, "DataVersion", Int) as u32;
    let size_x = *required_nbt!(nbt, "Width", Short) as u32;
    let size_y = *required_nbt!(nbt, "Height", Short) as u32;
    let size_z = *required_nbt!(nbt, "Length", Short) as u32;
    todo!()
}

pub fn serialize(schem: &Schematic, version: u32) -> Vec<u8> {
    todo!();
}
