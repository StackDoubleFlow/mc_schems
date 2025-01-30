use super::{BlockEntity, Blocks, Schematic, SchematicError, SchematicFormat};
use nbt::Value;
use std::collections::HashMap;

macro_rules! required_nbt {
    ($nbt:expr, $name:tt, $ty:ident) => {
        match $nbt.get($name) {
            Some(Value::$ty(value)) => value,
            Some(_) => return Err(SchematicError::MistypedField($name.to_owned())),
            None => return Err(SchematicError::MissingRequiredField($name.to_owned())),
        }
    };
}

macro_rules! typed_nbt {
    ($nbt:expr, $name:tt, $ty:ident) => {
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

    let data_name = match version {
        2 => "BlockData",
        3 => "Data",
        _ => unreachable!(),
    };
    let block_arr: Vec<u8> = required_nbt!(nbt, data_name, ByteArray)
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
    let nbt = match version {
        2 => &nbt.content,
        3 => required_nbt!(nbt, "Schematic", Compound),
        _ => {
            return Err(SchematicError::UnsupportedFormat(SchematicFormat::Sponge(
                version,
            )))
        }
    };

    let data_version = *required_nbt!(nbt, "DataVersion", Int) as u32;
    let size_x = *required_nbt!(nbt, "Width", Short) as u32;
    let size_y = *required_nbt!(nbt, "Height", Short) as u32;
    let size_z = *required_nbt!(nbt, "Length", Short) as u32;

    let mut metadata = typed_nbt!(nbt, "Metadata", Compound).cloned();
    let paste_offset = if version == 3 {
        let offset = required_nbt!(nbt, "Offset", IntArray);
        if offset.len() != 3 {
            return Err(SchematicError::MistypedField("Offset".to_owned()));
        }
        Some((offset[0], offset[1], offset[2]))
    } else if let Some(metadata) = &mut metadata {
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
    let origin = if version < 3 {
        typed_nbt!(nbt, "Offset", IntArray).map(|vec| (vec[0], vec[1], vec[2]))
    } else {
        None
    };

    let block_container = if version == 3 {
        &required_nbt!(nbt, "Blocks", Compound)
    } else {
        nbt
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

macro_rules! convert_or_err {
    ($nbt:expr, $name:tt, $ty:ident, $val:expr) => {
        $nbt.insert(
            $name.to_owned(),
            Value::$ty(
                $val.try_into()
                    .map_err(|_| SchematicError::InvalidValue($name.to_owned()))?,
            ),
        )
    };
}

fn write_block_container(
    version: u32,
    blocks: &Blocks,
    block_entities: &HashMap<(u32, u32, u32), BlockEntity>,
    nbt: &mut HashMap<String, Value>,
) {
    let mut palette = HashMap::new();
    for (idx, name) in blocks.palette.iter().enumerate() {
        palette.insert(name.to_string(), Value::Int(idx as i32));
    }
    nbt.insert("Palette".to_owned(), Value::Compound(palette));

    let mut bytes = Vec::new();
    for y in 0..blocks.size_y {
        for z in 0..blocks.size_z {
            for x in 0..blocks.size_x {
                let mut idx = blocks.get_block_id_at(x, y, z);
                // TODO: check max size for varint (5)
                loop {
                    let mut temp = (idx & 0b1111_1111) as u8;
                    idx >>= 7;
                    if idx != 0 {
                        temp |= 0b1000_0000;
                    }
                    bytes.push(temp as i8);
                    if idx == 0 {
                        break;
                    }
                }
            }
        }
    }
    let data_name = match version {
        2 => "BlockData",
        3 => "Data",
        _ => unreachable!(),
    };
    nbt.insert(data_name.to_owned(), Value::ByteArray(bytes));

    let mut nbt_block_entities = Vec::new();
    for (pos, block_entity) in block_entities {
        let mut data = block_entity.data.clone();
        data.insert("Id".to_owned(), nbt::Value::String(block_entity.id.clone()));
        let pos_arr = vec![pos.0 as i32, pos.1 as i32, pos.2 as i32];
        data.insert("Pos".to_owned(), nbt::Value::IntArray(pos_arr));
        nbt_block_entities.push(nbt::Value::Compound(data));
    }
    nbt.insert(
        "BlockEntities".to_owned(),
        nbt::Value::List(nbt_block_entities),
    );
}

pub fn serialize(schem: &Schematic, version: u32) -> Result<Vec<u8>, SchematicError> {
    let mut nbt = HashMap::new();

    nbt.insert("Version".to_owned(), Value::Int(version as i32));
    nbt.insert(
        "DataVersion".to_owned(),
        Value::Int(
            schem
                .data_version
                .ok_or_else(|| SchematicError::MissingRequiredField("DataVersion".to_owned()))?
                as i32,
        ),
    );
    convert_or_err!(nbt, "Width", Short, schem.blocks.size_x);
    convert_or_err!(nbt, "Height", Short, schem.blocks.size_y);
    convert_or_err!(nbt, "Length", Short, schem.blocks.size_z);

    // WorldEdit puts the paste offset into the metadata for version < 3, so we will do the same
    if version < 3 && schem.paste_offset.is_some() || schem.metadata.is_some() {
        let mut metadata = if let Some(metadata) = &schem.metadata {
            metadata.clone()
        } else {
            HashMap::new()
        };
        if version < 3 {
            if let Some(offset) = schem.paste_offset {
                metadata.insert("WEOffsetX".to_owned(), Value::Int(offset.0));
                metadata.insert("WEOffsetY".to_owned(), Value::Int(offset.1));
                metadata.insert("WEOffsetZ".to_owned(), Value::Int(offset.2));
            }
        }
    }

    if version == 3 {
        let offset = schem
            .paste_offset
            .ok_or_else(|| SchematicError::MissingRequiredField("Offset".to_owned()))?;
        nbt.insert(
            "Offset".to_owned(),
            nbt::Value::IntArray(vec![offset.0, offset.1, offset.2]),
        );
    }

    if version < 3 {
        // Worldedit encodes the origin as offset in v1 and v2 due to a misunderstanding of the
        // spec, so we'll replicate that behaviour
        if let Some(origin) = schem.origin {
            nbt.insert(
                "Offset".to_owned(),
                nbt::Value::IntArray(vec![origin.0, origin.1, origin.2]),
            );
        }
    }

    if version < 3 {
        write_block_container(version, &schem.blocks, &schem.block_entities, &mut nbt);
    } else {
        let mut container = HashMap::new();
        write_block_container(version, &schem.blocks, &schem.block_entities, &mut container);
        nbt.insert("Blocks".to_owned(), Value::Compound(container));
    };

    let root = match version {
        2 => nbt::Blob {
            content: nbt,
            title: String::new(),
        },
        3 => {
            let mut blob = nbt::Blob::new();
            blob.insert("Schematic", nbt::Value::Compound(nbt)).unwrap();
            blob
        }
        _ => unreachable!(),
    };
    let mut data = Vec::new();
    root.to_gzip_writer(&mut data)?;
    Ok(data)
}
