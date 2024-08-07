use super::{BlockEntity, Blocks, Schematic, SchematicError, SchematicFormat};
use nbt::Value;
use std::collections::HashMap;

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
    ($nbt:expr, $name:literal, $ty:ident) => {
        match $nbt.get($name) {
            Some(Value::$ty(value)) => Some(value),
            Some(_) => return Err(SchematicError::MistypedField($name.to_owned())),
            None => None,
        }
    };
}

fn read_block_container(
    version: u32,
    size_x: u32,
    size_y: u32,
    size_z: u32,
    nbt: &HashMap<String, Value>,
) -> Result<(Blocks, HashMap<(u32, u32, u32), BlockEntity>), SchematicError> {
    let mut blocks = Blocks::new(size_x, size_y, size_z, "minecraft:air");

    let nbt_palette = required_nbt!(nbt, "Palette", Compound);
    let mut palette = HashMap::new();
    for (name, value) in nbt_palette.iter() {
        let Value::Int(value) = value else {
            return Err(SchematicError::MistypedField(name.to_string()));
        };
        palette.insert(*value as u32, blocks.get_block_id_for(name));
    }

    let block_arr: Vec<u8> = required_nbt!(nbt, "BlockData", ByteArray)
        .iter()
        .map(|b| *b as u8)
        .collect();
    let mut i = 0;
    for y in 0..size_y {
        for z in 0..size_z {
            for x in 0..size_x {
                let mut blockstate_id = 0;
                // Max varint length is 5
                for varint_len in 0..=5 {
                    blockstate_id |= ((block_arr[i] & 127) as u32) << (varint_len * 7);
                    if (block_arr[i] & 128) != 128 {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                let id = palette[&blockstate_id];
                blocks.set_block_id_at(x, y, z, id);
            }
        }
    }

    let nbt_block_entities = typed_nbt!(nbt, "BlockEntities", List)
        .map(|l| l.as_slice())
        .unwrap_or_default();
    let mut block_entities = HashMap::new();
    for block_entity in nbt_block_entities {
        let Value::Compound(val) = block_entity else {
            return Err(SchematicError::MistypedField(
                if version == 1 {
                    "TileEntities"
                } else {
                    "BlockEntities"
                }
                .to_string(),
            ));
        };
        let pos_array = required_nbt!(val, "Pos", IntArray);
        let pos = (
            pos_array[0] as u32,
            pos_array[1] as u32,
            pos_array[2] as u32,
        );
        let id = required_nbt!(val, "Id", String);
        let mut data = val.clone();
        data.remove("Pos");
        data.remove("Id");

        block_entities.insert(
            pos,
            BlockEntity {
                id: id.clone(),
                data,
            },
        );
    }
    Ok((blocks, block_entities))
}

pub fn deserialize(nbt: &nbt::Blob, version: u32) -> Result<Schematic, SchematicError> {
    if version != 2 {
        return Err(SchematicError::UnsupportedFormat(SchematicFormat::Sponge(
            version,
        )));
    }
    let data_version = *required_nbt!(nbt, "DataVersion", Int) as u32;
    let size_x = *required_nbt!(nbt, "Width", Short) as u32;
    let size_y = *required_nbt!(nbt, "Height", Short) as u32;
    let size_z = *required_nbt!(nbt, "Length", Short) as u32;

    let mut metadata = typed_nbt!(nbt, "Metadata", Compound).cloned();
    let paste_offset = if let Some(metadata) = &mut metadata {
        // We're pretty relaxed about reading this since it's non-standard
        if metadata.contains_key("WEOffsetX") {
            let offset = Some((
                typed_nbt!(metadata, "WEOffsetX", Int)
                    .copied()
                    .unwrap_or_default(),
                typed_nbt!(metadata, "WEOffsetY", Int)
                    .copied()
                    .unwrap_or_default(),
                typed_nbt!(metadata, "WEOffsetZ", Int)
                    .copied()
                    .unwrap_or_default(),
            ));
            metadata.remove_entry("WEOffsetX");
            metadata.remove_entry("WEOffsetY");
            metadata.remove_entry("WEOffsetZ");
            offset
        } else {
            None
        }
    } else {
        None
    };

    if metadata.as_ref().is_some_and(HashMap::is_empty) {
        metadata = None;
    }

    // Worldedit encodes the origin as offset in v1 and v2 due to a misunderstanding of the spec
    let origin = typed_nbt!(nbt, "Offset", IntArray).map(|vec| (vec[0], vec[1], vec[2]));

    let block_container = if version == 3 {
        &required_nbt!(nbt, "Blocks", Compound)
    } else {
        &nbt.content
    };
    let (blocks, block_entities) =
        read_block_container(version, size_x, size_y, size_z, block_container)?;

    Ok(Schematic {
        blocks,
        data_version: Some(data_version),
        paste_offset,
        origin,
        // TODO
        biomes: None,
        block_entities,
        metadata,
    })
}

pub fn serialize(schem: &Schematic, version: u32) -> Vec<u8> {
    todo!();
}
