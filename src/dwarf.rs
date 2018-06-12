extern crate elf;
extern crate gimli;

use self::gimli::*;
use std::collections::HashMap;
use std::convert::*;
use std::default::Default;

#[derive(Debug, Clone, Default)]
pub struct DwarfLookup {
    struct_lookup: HashMap<usize, CStruct>,
    union_lookup: HashMap<usize, CUnion>,
    struct_name_lookup: HashMap<String, usize>,
    union_name_lookup: HashMap<String, usize>,
}

enum EntryKind {
    BaseType,
    Struct,
    TypeDef,
    Pointer,
    Array,
    Union,
    Enum,
    Const,
}

#[derive(Debug, Clone, Default)]
struct CEnum {
    name: String,
    byte_size: usize,
}

#[derive(Debug, Clone, Default)]
struct CArray {
    type_id: usize,
    count: usize,
}

#[derive(Debug, Clone, Default)]
pub struct CUnion {
    name: String,
    pub byte_size: usize,
    members: HashMap<String, CMember>,
}

#[derive(Debug, Clone, Default)]
struct CPointer {
    byte_size: usize,
    type_id: usize,
}

#[derive(Debug, Clone, Default)]
struct CConst {
    type_id: usize,
}

#[derive(Debug, Clone, Default)]
struct CBaseType {
    name: String,
    byte_size: usize,
}

#[derive(Debug, Clone, Default)]
pub struct CMember {
    name: String,
    pub byte_size: usize,
    pub byte_offset: usize,
    pub type_id: usize,
}

#[derive(Debug, Clone, Default)]
pub struct CStruct {
    name: String,
    pub byte_size: usize,
    members: HashMap<String, CMember>,
}

#[derive(Debug, Clone, Default)]
pub struct CTypeDef {
    name: String,
    type_id: usize,
}

pub fn parse_dwarf_file(file: String) -> DwarfLookup {
    let dwz_file = elf::File::open_path(file).expect("Failed to open file");
    let debug_info = dwz_file
        .get_section(".debug_info")
        .expect("Does not have .debug_info section");
    let debug_abbrev = dwz_file
        .get_section(".debug_abbrev")
        .expect("Does not have .debug_abbrev section");
    let debug_str = dwz_file
        .get_section(".debug_str")
        .expect("Does not have .debug_str section");

    parse_dwarf(
        &debug_info.data[..],
        &debug_abbrev.data[..],
        &debug_str.data[..],
    )
}

fn parse_dwarf(debug_info: &[u8], debug_abbrev: &[u8], debug_str: &[u8]) -> DwarfLookup {
    let debug_info = gimli::DebugInfo::new(debug_info, LittleEndian);
    let debug_abbrev = gimli::DebugAbbrev::new(debug_abbrev, LittleEndian);
    let debug_str = gimli::DebugStr::new(debug_str, LittleEndian);

    let mut iter = debug_info.units();

    let mut typedef_lookup: HashMap<usize, CTypeDef> = HashMap::new();
    let mut struct_lookup: HashMap<usize, CStruct> = HashMap::new();
    let mut basetype_lookup: HashMap<usize, CBaseType> = HashMap::new();
    let mut array_lookup: HashMap<usize, CArray> = HashMap::new();
    let mut pointer_lookup: HashMap<usize, CPointer> = HashMap::new();
    let mut union_lookup: HashMap<usize, CUnion> = HashMap::new();
    let mut enum_lookup: HashMap<usize, CEnum> = HashMap::new();
    let mut const_lookup: HashMap<usize, CConst> = HashMap::new();

    let mut kind_lookup: HashMap<usize, EntryKind> = HashMap::new();

    while let Some(unit) = iter.next().unwrap() {
        let abbrevs_for_unit = unit.abbreviations(&debug_abbrev).unwrap();
        let mut entries = unit.entries(&abbrevs_for_unit);

        let mut prev_entry_offset: Option<usize> = None;

        while let Ok(Some((_, entry))) = entries.next_dfs() {
            let tag = entry.tag();
            let entry_offset: usize = entry.offset().to_debug_info_offset(&unit).0;

            match tag {
                gimli::DW_TAG_typedef => {
                    let mut typedef: CTypeDef = Default::default();
                    let mut attrs = entry.attrs();
                    while let Ok(Some(attr)) = attrs.next() {
                        match attr.name() {
                            gimli::DW_AT_name => typedef.name = parse_attr_string(attr, debug_str),
                            gimli::DW_AT_type => typedef.type_id = parse_attr_at_type(attr, unit),
                            _ => (),
                        }
                    }
                    typedef_lookup.insert(entry_offset, typedef);
                    kind_lookup.insert(entry_offset, EntryKind::TypeDef);
                }

                gimli::DW_TAG_structure_type => {
                    let mut cstruct: CStruct = Default::default();
                    let mut attrs = entry.attrs();
                    let mut declaration = false;
                    while let Ok(Some(attr)) = attrs.next() {
                        match attr.name() {
                            gimli::DW_AT_name => cstruct.name = parse_attr_string(attr, debug_str),
                            gimli::DW_AT_byte_size => cstruct.byte_size = parse_attr_usize(attr),
                            gimli::DW_AT_bit_size => cstruct.byte_size = parse_attr_usize(attr) / 8,
                            gimli::DW_AT_declaration => {
                                if let gimli::AttributeValue::Flag(flag) = attr.value() {
                                    declaration = flag;
                                }
                            }
                            _ => (),
                        }
                    }
                    if !declaration {
                        prev_entry_offset = Some(entry_offset);
                        struct_lookup.insert(entry_offset, cstruct);
                        kind_lookup.insert(entry_offset, EntryKind::Struct);
                    } else {
                        prev_entry_offset = None;
                    }
                }

                gimli::DW_TAG_member => {
                    let mut cmember: CMember = Default::default();
                    let mut attrs = entry.attrs();
                    while let Ok(Some(attr)) = attrs.next() {
                        match attr.name() {
                            gimli::DW_AT_name => cmember.name = parse_attr_string(attr, debug_str),
                            gimli::DW_AT_data_member_location => {
                                cmember.byte_offset = parse_attr_usize(attr)
                            }
                            gimli::DW_AT_byte_size => cmember.byte_offset = parse_attr_usize(attr),
                            gimli::DW_AT_bit_size => {
                                cmember.byte_offset = parse_attr_usize(attr) / 8
                            }
                            gimli::DW_AT_type => cmember.type_id = parse_attr_at_type(attr, unit),
                            _ => (),
                        }
                    }
                    if prev_entry_offset.is_some() {
                        let kind = &kind_lookup[&prev_entry_offset.unwrap()];
                        match *kind {
                            EntryKind::Struct => {
                                let cstruct =
                                    struct_lookup.get_mut(&prev_entry_offset.unwrap()).unwrap();
                                cstruct.members.insert(cmember.name.clone(), cmember);
                            }
                            EntryKind::Union => {
                                let cunion =
                                    union_lookup.get_mut(&prev_entry_offset.unwrap()).unwrap();
                                cunion.members.insert(cmember.name.clone(), cmember);
                            }
                            _ => (),
                        }
                    }
                }

                gimli::DW_TAG_base_type => {
                    let mut cbasetype: CBaseType = Default::default();
                    let mut attrs = entry.attrs();
                    while let Ok(Some(attr)) = attrs.next() {
                        match attr.name() {
                            gimli::DW_AT_name => {
                                cbasetype.name = parse_attr_string(attr, debug_str)
                            }
                            gimli::DW_AT_byte_size => cbasetype.byte_size = parse_attr_usize(attr),
                            gimli::DW_AT_bit_size => {
                                cbasetype.byte_size = parse_attr_usize(attr) / 8
                            }
                            _ => (),
                        }
                    }
                    basetype_lookup.insert(entry_offset, cbasetype);
                    kind_lookup.insert(entry_offset, EntryKind::BaseType);
                    prev_entry_offset = None;
                }

                gimli::DW_TAG_pointer_type => {
                    let mut cpointer: CPointer = Default::default();
                    let mut attrs = entry.attrs();
                    while let Ok(Some(attr)) = attrs.next() {
                        match attr.name() {
                            gimli::DW_AT_type => cpointer.type_id = parse_attr_at_type(attr, unit),
                            gimli::DW_AT_byte_size => cpointer.byte_size = parse_attr_usize(attr),
                            _ => (),
                        }
                    }
                    pointer_lookup.insert(entry_offset, cpointer);
                    kind_lookup.insert(entry_offset, EntryKind::Pointer);
                    prev_entry_offset = None;
                }

                gimli::DW_TAG_array_type => {
                    let mut carray: CArray = Default::default();
                    let mut attrs = entry.attrs();
                    while let Ok(Some(attr)) = attrs.next() {
                        if gimli::DW_AT_type == attr.name() {
                            carray.type_id = parse_attr_at_type(attr, unit);
                        }
                    }
                    prev_entry_offset = Some(entry_offset);
                    array_lookup.insert(entry_offset, carray);
                    kind_lookup.insert(entry_offset, EntryKind::Array);
                }

                gimli::DW_TAG_subrange_type => {
                    let mut attrs = entry.attrs();
                    let mut upper_bound = 0;
                    while let Ok(Some(attr)) = attrs.next() {
                        if gimli::DW_AT_upper_bound == attr.name() {
                            upper_bound = parse_attr_usize(attr);
                        }
                    }
                    if prev_entry_offset.is_some() {
                        let carray = array_lookup.get_mut(&prev_entry_offset.unwrap()).unwrap();
                        carray.count = upper_bound + 1;
                    }
                    prev_entry_offset = None;
                }

                gimli::DW_TAG_union_type => {
                    let mut cunion: CUnion = Default::default();
                    let mut attrs = entry.attrs();
                    let mut declaration = false;
                    while let Ok(Some(attr)) = attrs.next() {
                        match attr.name() {
                            gimli::DW_AT_name => cunion.name = parse_attr_string(attr, debug_str),
                            gimli::DW_AT_byte_size => cunion.byte_size = parse_attr_usize(attr),
                            gimli::DW_AT_bit_size => cunion.byte_size = parse_attr_usize(attr) / 8,
                            gimli::DW_AT_declaration => {
                                if let gimli::AttributeValue::Flag(flag) = attr.value() {
                                    declaration = flag;
                                }
                            }
                            _ => (),
                        }
                    }
                    if !declaration {
                        prev_entry_offset = Some(entry_offset);
                        union_lookup.insert(entry_offset, cunion);
                        kind_lookup.insert(entry_offset, EntryKind::Union);
                    } else {
                        prev_entry_offset = None;
                    }
                }

                gimli::DW_TAG_enumeration_type => {
                    let mut cenum: CEnum = Default::default();
                    let mut attrs = entry.attrs();
                    while let Ok(Some(attr)) = attrs.next() {
                        match attr.name() {
                            gimli::DW_AT_name => cenum.name = parse_attr_string(attr, debug_str),
                            gimli::DW_AT_byte_size => cenum.byte_size = parse_attr_usize(attr),
                            _ => (),
                        }
                    }
                    enum_lookup.insert(entry_offset, cenum);
                    kind_lookup.insert(entry_offset, EntryKind::Enum);
                }

                gimli::DW_TAG_const_type => {
                    let mut cconst: CConst = Default::default();
                    let mut attrs = entry.attrs();
                    while let Ok(Some(attr)) = attrs.next() {
                        if gimli::DW_AT_type == attr.name() {
                            cconst.type_id = parse_attr_at_type(attr, unit);
                        }
                    }
                    const_lookup.insert(entry_offset, cconst);
                    kind_lookup.insert(entry_offset, EntryKind::Const);
                }
                _ => {
                    // println!("Tag: {:?}", entry.tag());
                    // let mut attrs = entry.attrs();
                    // while let Ok(Some(attr)) = attrs.next() {
                    //     // println!("attr {:?} => {:?}", attr.name(), attr.value());
                    // }
                }
            }
        }
    }

    let mut struct_name_lookup: HashMap<String, usize> = HashMap::new();
    let mut union_name_lookup: HashMap<String, usize> = HashMap::new();
    let struct_lookup_clone = struct_lookup.clone();
    let union_lookup_clone = union_lookup.clone();

    for (id, cstruct) in &mut struct_lookup {
        for member in cstruct.members.values_mut() {
            let mut size = member.byte_size;
            if size == 0 {
                size = get_type_size(
                    member.type_id,
                    &kind_lookup,
                    &typedef_lookup,
                    &struct_lookup_clone,
                    &basetype_lookup,
                    &array_lookup,
                    &pointer_lookup,
                    &union_lookup,
                    &enum_lookup,
                    &const_lookup,
                );
            }
            member.byte_size = size;
        }
        struct_name_lookup.insert(cstruct.name.clone(), *id);
    }

    for (id, cunion) in &mut union_lookup {
        for member in cunion.members.values_mut() {
            let mut size = member.byte_size;
            if size == 0 {
                size = get_type_size(
                    member.type_id,
                    &kind_lookup,
                    &typedef_lookup,
                    &struct_lookup,
                    &basetype_lookup,
                    &array_lookup,
                    &pointer_lookup,
                    &union_lookup_clone,
                    &enum_lookup,
                    &const_lookup,
                );
            }
            member.byte_size = size;
        }
        union_name_lookup.insert(cunion.name.clone(), *id);
    }

    DwarfLookup {
        struct_lookup,
        union_lookup,
        struct_name_lookup,
        union_name_lookup,
    }
}

fn get_type_size(
    id: usize,
    kind_lookup: &HashMap<usize, EntryKind>,
    typedef_lookup: &HashMap<usize, CTypeDef>,
    struct_lookup: &HashMap<usize, CStruct>,
    basetype_lookup: &HashMap<usize, CBaseType>,
    array_lookup: &HashMap<usize, CArray>,
    pointer_lookup: &HashMap<usize, CPointer>,
    union_lookup: &HashMap<usize, CUnion>,
    enum_lookup: &HashMap<usize, CEnum>,
    const_lookup: &HashMap<usize, CConst>,
) -> usize {
    let mut id = id;
    let mut tries = 0;
    let mut count = 1;

    loop {
        tries += 1;
        if tries > 10 {
            return 0;
        }

        let skind = kind_lookup.get(&id);
        if skind.is_none() {
            return 0;
        }
        match *skind.unwrap() {
            EntryKind::BaseType => return count * basetype_lookup.get(&id).unwrap().byte_size,
            EntryKind::Enum => return count * enum_lookup.get(&id).unwrap().byte_size,
            EntryKind::Pointer => return count * pointer_lookup.get(&id).unwrap().byte_size,
            EntryKind::Struct => return count * struct_lookup.get(&id).unwrap().byte_size,
            EntryKind::Union => return count * union_lookup.get(&id).unwrap().byte_size,
            EntryKind::TypeDef => id = typedef_lookup.get(&id).unwrap().type_id,
            EntryKind::Const => id = const_lookup.get(&id).unwrap().type_id,
            EntryKind::Array => {
                let arr = array_lookup.get(&id).unwrap();
                count = arr.count;
                id = arr.type_id;
            }
        }
    }
}

fn parse_attr_at_type<'input, Endian>(
    attr: gimli::Attribute<EndianBuf<'input, Endian>>,
    unit: gimli::CompilationUnitHeader<EndianBuf<'input, Endian>>,
) -> usize
where
    Endian: gimli::Endianity,
{
    match attr.value() {
        gimli::AttributeValue::DebugInfoRef(offset) => offset.0,
        gimli::AttributeValue::UnitRef(unit_offset) => unit_offset.to_debug_info_offset(&unit).0,
        _ => 0,
    }
}

fn parse_attr_usize<Endian>(attr: gimli::Attribute<EndianBuf<Endian>>) -> usize
where
    Endian: gimli::Endianity,
{
    match attr.udata_value() {
        Some(u) => u as usize,
        None => panic!("udata"),
    }
}

fn parse_attr_string<'input, Endian>(
    attr: gimli::Attribute<EndianBuf<'input, Endian>>,
    debug_str: gimli::DebugStr<EndianBuf<'input, Endian>>,
) -> String
where
    Endian: gimli::Endianity,
{
    match attr.value() {
        gimli::AttributeValue::String(s) => s.to_string_lossy().to_string(),
        gimli::AttributeValue::DebugStrRef(o) => match debug_str.get_str(o) {
            Ok(s) => s.to_string_lossy().to_string(),
            Err(_) => String::new(),
        },
        _ => String::new(),
    }
}

impl DwarfLookup {
    pub fn find_struct(&self, name: &str) -> Option<&CStruct> {
        match self.struct_name_lookup.get(&String::from(name)) {
            None => None,
            Some(&id) => self.find_struct_by_id(id),
        }
    }

    pub fn find_union(&self, name: &str) -> Option<&CUnion> {
        match self.union_name_lookup.get(&String::from(name)) {
            None => None,
            Some(&id) => self.find_union_by_id(id),
        }
    }

    pub fn find_struct_by_id(&self, id: usize) -> Option<&CStruct> {
        self.struct_lookup.get(&id)
    }

    pub fn find_union_by_id(&self, id: usize) -> Option<&CUnion> {
        self.union_lookup.get(&id)
    }
}

impl CStruct {
    pub fn find_member(&self, name: &str) -> Option<&CMember> {
        self.members.get(&String::from(name))
    }
}

impl CUnion {
    pub fn find_member(&self, name: &str) -> Option<&CMember> {
        self.members.get(&String::from(name))
    }
}
