//! This library provides a convenient way to read, write, and convert Minecraft schematic files of
//! various formats.

mod sponge;

use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SchematicError {
    /// The format of the schematic data could not be recongnized as one of the supported types.
    #[error("unrecongnized schematic format")]
    UnrecognizedFormat,
    /// The format of the schematic data could be recognized but (de)serialization is unsupported.
    #[error("unsupported schematic format")]
    UnsupportedFormat(SchematicFormat),
}

/// Types of schematic formats used by Schematica
#[derive(Debug, Clone, Copy)]
pub enum SchematicaFormat {
    Structure,
    Alpha,
}

#[derive(Debug, Clone, Copy)]
/// The known schematic formats. Not that not all of these schematic formats are supported by this
/// library.
pub enum SchematicFormat {
    /// The Sponge Schematic Format.
    ///
    /// Specification:
    /// - [Version 1](https://github.com/SpongePowered/Schematic-Specification/blob/master/versions/schematic-1.md)
    /// - [Version 2](https://github.com/SpongePowered/Schematic-Specification/blob/master/versions/schematic-2.md)
    /// - [Version 3](https://github.com/SpongePowered/Schematic-Specification/blob/master/versions/schematic-3.md)
    Sponge(u32),
    /// The Litematica Schematic Format.
    ///
    /// Unlike Sponge, this schematic format does not have a clear specification.
    Litematica(u32),
    /// The Schematica Schematic Format.
    ///
    /// Unlike Sponge, this schematic format does not have a clear specification.
    /// - [`SchematicaFormat::Alpha`](https://github.com/Lunatrius/Schematica/blob/master/src/main/java/com/github/lunatrius/schematica/world/schematic/SchematicAlpha.java)
    /// - [`SchematicaFormat::Structure`](https://github.com/Lunatrius/Schematica/blob/master/src/main/java/com/github/lunatrius/schematica/world/schematic/SchematicStructure.java)
    Schematica(SchematicaFormat),
}

/// A simple fixed-size block storage for dealing with schematic files.
pub struct Blocks {
    palette: Vec<String>,
    palette_map: HashMap<String, u32>,
    indices: Vec<u32>,
    size_x: u32,
    size_y: u32,
    size_z: u32,
}

impl Blocks {
    pub fn new(size_x: u32, size_y: u32, size_z: u32, initial_block: &str) -> Self {
        Self {
            palette: vec![initial_block.to_owned()],
            indices: vec![0; (size_x * size_y * size_z) as usize],
            palette_map: {
                let mut map = HashMap::new();
                map.insert(initial_block.to_owned(), 0);
                map
            },
            size_x,
            size_y,
            size_z,
        }
    }

    /// Get the size of this container (x, y, z)
    pub fn size(&self) -> (u32, u32, u32) {
        (self.size_x, self.size_y, self.size_z)
    }

    /// Panic if a position is out of bounds
    fn bounds_check(&self, pos_x: u32, pos_y: u32, pos_z: u32) {
        if pos_x >= self.size_x || pos_y >= self.size_y || pos_z >= self.size_z {
            panic!(
                "position ({pos_x}, {pos_y}, {pos_z}) out of bounds for container with size ({:?})",
                self.size()
            );
        }
    }

    fn block_index_at(&self, pos_x: u32, pos_y: u32, pos_z: u32) -> usize {
        ((pos_x * self.size_y * self.size_z) + (pos_y * self.size_z) + pos_z) as usize
    }

    /// Get the palette index for a block at a position
    pub fn get_block_id_at(&self, pos_x: u32, pos_y: u32, pos_z: u32) -> u32 {
        self.bounds_check(pos_x, pos_y, pos_z);
        self.indices[self.block_index_at(pos_x, pos_y, pos_z)]
    }

    /// Get the name of a block at a position
    pub fn get_block_at(&self, pos_x: u32, pos_y: u32, pos_z: u32) -> &str {
        let id = self.get_block_id_at(pos_x, pos_y, pos_z);
        &self.palette[id as usize]
    }

    /// Get the palette index for a block name. If the block is not already in the palette, it will
    /// be added.
    pub fn get_block_id_for(&mut self, block: &str) -> u32 {
        match self.palette_map.get(block) {
            Some(id) => *id,
            None => {
                let next_id = self.palette.len() as u32;
                self.palette.push(block.to_owned());
                self.palette_map.insert(block.to_owned(), next_id);
                next_id
            }
        }
    }

    /// Set the palette index for a block at a position
    pub fn set_block_id_at(&mut self, pos_x: u32, pos_y: u32, pos_z: u32, id: u32) {
        self.bounds_check(pos_x, pos_y, pos_z);
        let idx = self.block_index_at(pos_x, pos_y, pos_z);
        self.indices[idx] = id;
    }

    /// Set the name of a block at a position
    pub fn set_block_at(&mut self, pos_x: u32, pos_y: u32, pos_z: u32, block: &str) {
        let id = self.get_block_id_for(block);
        self.set_block_id_at(pos_x, pos_y, pos_z, id);
    }
}

/// A schematic file
pub struct Schematic {
    pub blocks: Blocks,
    pub block_entities: HashMap<(u32, u32, u32), HashMap<String, nbt::Value>>,
}

impl Schematic {
    /// Get the size of this schematic (x, y, z)
    pub fn size(&self) -> (u32, u32, u32) {
        self.blocks.size()
    }

    /// Deserialize a schematic from a raw byte slice.
    ///
    /// This function will attempt to detect which format the schematic is encoded in. If the format
    /// cannot be recognized, [`SchematicError::UnrecognizedFormat`] is returned. Not all
    /// schematic formats representable with [`SchematicFormat`] are deserializable. In that case,
    /// [`SchematicError::UnsupportedFormat`] is returned.
    pub fn deserialize(data: &[u8]) -> Result<Schematic, SchematicError> {
        todo!()
    }

    /// Serialize a schematic into raw bytes.
    ///
    /// Not all schematic formats representable with [`SchematicFormat`] are serializable. In that
    /// case, [`SchematicError::UnsupportedFormat`] is returned.
    pub fn serialize(&self, format: SchematicFormat) -> Result<Vec<u8>, SchematicError> {
        let data = match format {
            SchematicFormat::Sponge(version @ 3) => sponge::serialize(self, version),
            _ => return Err(SchematicError::UnsupportedFormat(format)),
        };
        Ok(data)
    }
}
