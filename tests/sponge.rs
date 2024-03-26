use mc_schems::Schematic;

#[test]
fn sponge_v2() {
    let bytes = include_bytes!("sponge_v2.schem");
    let schem = Schematic::deserialize(bytes).unwrap();

    assert_eq!(schem.blocks.size(), (2, 2, 2));
    assert!(schem.block_entities.is_empty());
    assert!(schem.biomes.is_none());
    assert_eq!(schem.paste_offset, Some((1, 0, 1)));
    assert_eq!(schem.origin, Some((1, 0, 2)));
    assert!(schem.metadata.is_none());
}
