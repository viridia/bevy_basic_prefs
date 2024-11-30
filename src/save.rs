use std::fs;

use bevy::{
    ecs::world::Command,
    prelude::*,
    reflect::{Enum, EnumInfo, ReflectFromPtr, ReflectRef, TypeInfo, VariantType},
};

use crate::{PreferencesChanged, PreferencesDir, PreferencesGroup, PreferencesKey};

#[derive(Default, PartialEq)]
pub enum SavePreferences {
    /// Save prefernces only if they have changed (based on [`PreferencesChanged` resource]).
    #[default]
    IfChanged,
    /// Save preferences unconditionally.
    Always,
}

impl Command for SavePreferences {
    fn apply(self, world: &mut World) {
        let mut changed = world.get_resource_mut::<PreferencesChanged>().unwrap();
        if changed.0 || self == SavePreferences::Always {
            changed.0 = false;
            let prefs_dir = world.get_resource::<PreferencesDir>().unwrap();
            let registry = world.get_resource::<AppTypeRegistry>().unwrap();
            // let asset_server = world.get_resource::<AssetServer>();
            let registry_read = registry.read();
            let prefs_file_new = prefs_dir.0.join("prefs.toml.new");
            let prefs_file = prefs_dir.0.join("prefs.toml");
            let mut table = toml::Table::new();
            for (res, _) in world.iter_resources() {
                if let Some(tid) = res.type_id() {
                    if let Some(treg) = registry_read.get(tid) {
                        match treg.type_info() {
                            bevy::reflect::TypeInfo::Struct(stty) => {
                                let group_attr = stty.custom_attributes().get::<PreferencesGroup>();
                                let key_attr = stty.custom_attributes().get::<PreferencesKey>();
                                if group_attr.is_some() || key_attr.is_some() {
                                    let ptr = world.get_resource_by_id(res.id()).unwrap();
                                    let reflect_from_ptr = treg.data::<ReflectFromPtr>().unwrap();
                                    let ReflectRef::Struct(st) =
                                        unsafe { reflect_from_ptr.as_reflect(ptr) }.reflect_ref()
                                    else {
                                        panic!("Expected Struct");
                                    };
                                    maybe_save_struct(st, group_attr, key_attr, &mut table);
                                }
                            }
                            bevy::reflect::TypeInfo::TupleStruct(tsty) => {
                                let group_attr = tsty.custom_attributes().get::<PreferencesGroup>();
                                let key_attr = tsty.custom_attributes().get::<PreferencesKey>();
                                let ptr = world.get_resource_by_id(res.id()).unwrap();
                                let reflect_from_ptr = treg.data::<ReflectFromPtr>().unwrap();
                                let ReflectRef::TupleStruct(tuple_struct) =
                                    unsafe { reflect_from_ptr.as_reflect(ptr) }.reflect_ref()
                                else {
                                    panic!("Expected TupleStruct");
                                };
                                if group_attr.is_some() || key_attr.is_some() {
                                    maybe_save_tuple_struct(
                                        tuple_struct,
                                        group_attr,
                                        key_attr,
                                        &mut table,
                                    );
                                } else if tsty
                                    .type_path()
                                    .starts_with("bevy_state::state::resources::State<")
                                {
                                    let state_reflect = tuple_struct.field(0).unwrap();
                                    let state_info =
                                        state_reflect.get_represented_type_info().unwrap();
                                    let field_reflect_ref = state_reflect.reflect_ref();
                                    match (state_info, field_reflect_ref) {
                                        (TypeInfo::Struct(_), ReflectRef::Struct(_)) => todo!(),
                                        (TypeInfo::TupleStruct(_), ReflectRef::TupleStruct(_)) => {
                                            todo!()
                                        }
                                        (TypeInfo::Enum(enum_ty), ReflectRef::Enum(enum_ref)) => {
                                            maybe_save_enum(enum_ty, enum_ref, &mut table);
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            bevy::reflect::TypeInfo::Enum(ety) => {
                                if let Some(_group) =
                                    ety.custom_attributes().get::<PreferencesGroup>()
                                {
                                    warn!("Preferences: Enums not supported yet: {}", res.name());
                                } else if let Some(_key) =
                                    ety.custom_attributes().get::<PreferencesKey>()
                                {
                                    warn!("Preferences: Enums not supported yet: {}", res.name());
                                }
                                // warn!("Preferences: Enums not supported yet: {}", res.name());
                            }

                            // Other types cannot be preferences since they don't have attributes.
                            _ => {}
                        }
                    }
                    // println!("Saving preferences for {:?}", res.name());
                }
            }

            // Recursively create the preferences directory if it doesn't exist.
            let mut dir_builder = std::fs::DirBuilder::new();
            dir_builder.recursive(true);
            if let Err(e) = dir_builder.create(prefs_dir.0.clone()) {
                warn!("Could not create preferences directory: {:?}", e);
                return;
            }

            // Write to temporary file.
            if let Err(e) = fs::write(&prefs_file_new, table.to_string()) {
                warn!("Could not write preferences file: {:?}", e);
                return;
            }

            // Replace old prefs file with new one.
            if let Err(e) = fs::rename(&prefs_file_new, prefs_file) {
                warn!("Could not save preferences file: {:?}", e);
            }

            // info!("Saved: {:?}", prefs_file);
            // println!("Preferences:\n{}\n", table);
        }
    }
}

fn maybe_save_struct(
    strct: &dyn Struct,
    group_attr: Option<&PreferencesGroup>,
    key_attr: Option<&PreferencesKey>,
    table: &mut toml::Table,
) {
    if let Some(group) = group_attr {
        let group = table
            .entry(group.0.to_string())
            .or_insert(toml::Value::Table(toml::Table::new()))
            .as_table_mut()
            .unwrap();
        if let Some(_key) = key_attr {
            todo!();
        } else {
            // TODO: Need to derive key name from tuple struct name
            save_struct(strct, group);
        }
    } else if let Some(_key) = key_attr {
        // save_struct(strct, key.0, table);
        todo!();
    }
}

fn save_struct(strct: &dyn Struct, table: &mut toml::Table) {
    for i in 0..strct.field_len() {
        let field_reflect = strct.field_at(i).unwrap();
        match field_reflect.reflect_ref() {
            ReflectRef::Struct(_) => todo!(),
            ReflectRef::TupleStruct(_) => todo!(),
            ReflectRef::Tuple(_) => todo!(),
            ReflectRef::List(_) => todo!(),
            ReflectRef::Array(_) => todo!(),
            ReflectRef::Map(_) => todo!(),
            ReflectRef::Set(_) => todo!(),
            ReflectRef::Enum(_) | ReflectRef::Opaque(_) => {
                store_prop(field_reflect, strct.name_at(i).unwrap(), table);
            }
        }
    }
}

fn maybe_save_tuple_struct(
    tuple_struct: &dyn TupleStruct,
    group_attr: Option<&PreferencesGroup>,
    key_attr: Option<&PreferencesKey>,
    table: &mut toml::Table,
) {
    if let Some(group) = group_attr {
        let group = table
            .entry(group.0.to_string())
            .or_insert(toml::Value::Table(toml::Table::new()))
            .as_table_mut()
            .unwrap();
        if let Some(key) = key_attr {
            save_tuple_struct(tuple_struct, key.0, group);
        } else {
            // TODO: Need to derive key name from tuple struct name
            todo!();
        }
    } else if let Some(key) = key_attr {
        save_tuple_struct(tuple_struct, key.0, table);
    }
}

fn save_tuple_struct(tuple_struct: &dyn TupleStruct, key: &'static str, table: &mut toml::Table) {
    if tuple_struct.field_len() == 1 {
        let field_reflect = tuple_struct.field(0).unwrap();
        match field_reflect.reflect_ref() {
            ReflectRef::Struct(_) => todo!(),
            ReflectRef::TupleStruct(_) => todo!(),
            ReflectRef::Tuple(_) => todo!(),
            ReflectRef::List(_) => todo!(),
            ReflectRef::Array(_) => todo!(),
            ReflectRef::Map(_) => todo!(),
            ReflectRef::Set(_) => todo!(),
            ReflectRef::Enum(_) | ReflectRef::Opaque(_) => {
                store_prop(field_reflect, key, table);
            }
        }
    }
}

fn maybe_save_enum(enum_ty: &EnumInfo, enum_ref: &dyn Enum, table: &mut toml::Table) {
    let group_attr = enum_ty.custom_attributes().get::<PreferencesGroup>();
    let key_attr = enum_ty.custom_attributes().get::<PreferencesKey>();
    if let Some(group) = group_attr {
        let group = table
            .entry(group.0.to_string())
            .or_insert(toml::Value::Table(toml::Table::new()))
            .as_table_mut()
            .unwrap();
        if let Some(key) = key_attr {
            save_enum(enum_ref, key.0, group);
        } else {
            // TODO: Need to derive key name from tuple struct name
            todo!();
        }
    } else if let Some(key) = key_attr {
        save_enum(enum_ref, key.0, table);
    }
}

fn save_enum(enum_ref: &dyn Enum, key: &'static str, table: &mut toml::Table) {
    if enum_ref.variant_type() != VariantType::Unit {
        todo!("Figure out how to encode non-unit enums in TOML");
    }
    let v = toml::Value::String(enum_ref.variant_name().to_string());
    table.insert(key.to_string(), v);
}

/// Encode a reflected property and store it in the table with the given key.
fn store_prop(value: &dyn PartialReflect, key: &str, table: &mut toml::Table) {
    match value.reflect_ref() {
        ReflectRef::Struct(st) => {
            let mut field_table = toml::Table::new();
            save_struct(st, &mut field_table);
            table.insert(key.to_string(), toml::Value::Table(field_table));
        }

        ReflectRef::TupleStruct(_) => todo!(),
        ReflectRef::Tuple(_) => todo!(),
        ReflectRef::List(_) => todo!(),
        ReflectRef::Array(_) => todo!(),
        ReflectRef::Map(_) => todo!(),
        ReflectRef::Set(_) => todo!(),

        ReflectRef::Enum(en) => {
            let type_path = value.get_represented_type_info().unwrap().type_path();
            if type_path.starts_with("core::option::Option") {
                // None values just leave out the key.
                if en.variant_name() == "Some" {
                    let some_value = en.field_at(0).unwrap();
                    store_prop(some_value, key, table);
                }
            } else {
                warn!("Preferences: Unsupported enum type: {:?}", type_path);
            }
        }

        ReflectRef::Opaque(val) => {
            if let Some(f) = value.try_downcast_ref::<f32>() {
                let v = toml::Value::Float(*f as f64);
                table.insert(key.to_string(), v);
            } else if let Some(f) = value.try_downcast_ref::<f64>() {
                let v = toml::Value::Float(*f);
                table.insert(key.to_string(), v);
            } else if let Some(i) = value.try_downcast_ref::<i8>() {
                let v = toml::Value::Integer(*i as i64);
                table.insert(key.to_string(), v);
            } else if let Some(i) = value.try_downcast_ref::<i16>() {
                let v = toml::Value::Integer(*i as i64);
                table.insert(key.to_string(), v);
            } else if let Some(i) = value.try_downcast_ref::<i32>() {
                let v = toml::Value::Integer(*i as i64);
                table.insert(key.to_string(), v);
            } else if let Some(i) = value.try_downcast_ref::<i64>() {
                let v = toml::Value::Integer(*i);
                table.insert(key.to_string(), v);
            } else if let Some(i) = value.try_downcast_ref::<u8>() {
                let v = toml::Value::Integer(*i as i64);
                table.insert(key.to_string(), v);
            } else if let Some(i) = value.try_downcast_ref::<u16>() {
                let v = toml::Value::Integer(*i as i64);
                table.insert(key.to_string(), v);
            } else if let Some(i) = value.try_downcast_ref::<u32>() {
                let v = toml::Value::Integer(*i as i64);
                table.insert(key.to_string(), v);
            } else if let Some(i) = value.try_downcast_ref::<u64>() {
                if *i <= i64::MAX as u64 {
                    let v = toml::Value::Integer(*i as i64);
                    table.insert(key.to_string(), v);
                } else {
                    warn!("Preferences: u64 value too large: {}", i);
                }
            } else if let Some(i) = value.try_downcast_ref::<usize>() {
                if *i <= i64::MAX as usize {
                    let v = toml::Value::Integer(*i as i64);
                    table.insert(key.to_string(), v);
                } else {
                    warn!("Preferences: usize value too large: {}", i);
                }
            } else if let Some(s) = value.try_downcast_ref::<String>() {
                let v = toml::Value::String(s.clone());
                table.insert(key.to_string(), v);
            } else {
                warn!("Preferences: Unsupported type: {:?}", val);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use toml::Table;

    #[derive(Reflect)]
    struct TestStruct {
        field1: f32,
        field2: String,
    }

    #[test]
    fn test_store_prop_f32() {
        let mut table = Table::new();
        let value: &dyn PartialReflect = &42.0f32;
        store_prop(value, "test_f32", &mut table);
        assert_eq!(table.get("test_f32").unwrap().as_float().unwrap(), 42.0);
    }

    #[test]
    fn test_store_prop_string() {
        let mut table = Table::new();
        let value: &dyn PartialReflect = &"test_string".to_string();
        store_prop(value, "test_string", &mut table);
        assert_eq!(
            table.get("test_string").unwrap().as_str().unwrap(),
            "test_string"
        );
    }

    #[test]
    fn test_store_prop_struct() {
        let mut table = Table::new();
        let test_struct = TestStruct {
            field1: 3.1,
            field2: "hello".to_string(),
        };
        let value: &dyn PartialReflect = &test_struct;
        store_prop(value, "test_struct", &mut table);
        assert!(table.get("test_struct").is_some());
        let struct_table = table.get("test_struct").unwrap().as_table().unwrap();
        // assert_eq!(struct_table.get("field1").unwrap().as_float().unwrap(), 3.1);
        assert_eq!(
            struct_table.get("field2").unwrap().as_str().unwrap(),
            "hello"
        );
    }

    #[test]
    fn test_store_prop_option_some() {
        let mut table = Table::new();
        let value: &dyn PartialReflect = &Some(42i32);
        store_prop(value, "test_option", &mut table);
        assert_eq!(table.get("test_option").unwrap().as_integer().unwrap(), 42);
    }

    #[test]
    fn test_store_prop_option_none() {
        let mut table = Table::new();
        let value: &dyn PartialReflect = &Option::<i32>::None;
        store_prop(value, "test_option", &mut table);
        assert!(table.get("test_option").is_none());
    }
}
