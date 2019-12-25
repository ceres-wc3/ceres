use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::rc::Rc;

use serde::{Deserialize, Serialize};

use crate::metadata::MetadataStore;
use crate::ObjectId;
use crate::ObjectKind;
use crate::parser::profile;
use crate::parser::slk;
use crate::object::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct ObjectStore {
    objects: BTreeMap<ObjectId, Rc<RefCell<Object>>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ObjectStoreStock {
    objects: BTreeMap<ObjectId, Object>,
}

impl Default for ObjectStore {
    fn default() -> ObjectStore {
        ObjectStore {
            objects: Default::default(),
        }
    }
}

impl Default for ObjectStoreStock {
    fn default() -> ObjectStoreStock {
        ObjectStoreStock {
            objects: Default::default(),
        }
    }
}

impl ObjectStore {
    pub fn objects(&self) -> impl Iterator<Item = &Rc<RefCell<Object>>> {
        self.objects.values()
    }

    pub fn object(&self, id: ObjectId) -> Option<&Rc<RefCell<Object>>> {
        self.objects.get(&id)
    }

    pub fn insert_object(&mut self, object: Object) {
        self.objects
            .insert(object.id(), Rc::new(RefCell::new(object)));
    }

    pub fn remove_object(&mut self, id: ObjectId) {
        self.objects.remove(&id);
    }

    pub fn add_from(&mut self, other: &ObjectStore) {
        for (id, other_object) in &other.objects {
            if let Some(object) = self.objects.get_mut(&id) {
                object.borrow_mut().add_from(&other_object.borrow());
            } else {
                let cloned = other_object.borrow().clone();
                self.objects.insert(*id, Rc::new(RefCell::new(cloned)));
            }
        }
    }

    fn insert_slk_row<'src>(
        &mut self,
        kind: ObjectKind,
        row: slk::Row<'src>,
        legend: &slk::Legend<'src>,
        metadata: &MetadataStore,
    ) -> Option<()> {
        let id = row
            .cells
            .get(0)
            .and_then(|c| c.value().as_str())
            .and_then(|id| ObjectId::from_bytes(id.as_bytes()))?;

        let object = if kind == ObjectKind::empty() {
            self.objects.get_mut(&id)?
        } else {
            self.objects
                .entry(id)
                .or_insert_with(|| Rc::new(RefCell::new(Object::new(id, kind))))
        };

        for (value, name) in row
            .cells
            .iter()
            .filter_map(|cell| legend.name_by_cell(&cell).map(|name| (cell.value(), name)))
        {
            object.borrow_mut().process_slk_field(value, name, metadata);
        }

        Some(())
    }

    fn insert_func_entry(&mut self, entry: profile::Entry, metadata: &MetadataStore) -> Option<()> {
        let id = ObjectId::from_bytes(entry.id.as_bytes())?;
        let object = self.objects.get_mut(&id)?;

        for (key, values) in entry.values {
            for (index, value) in values.split(',').enumerate() {
                object
                    .borrow_mut()
                    .process_func_field(key, value, index as i8, metadata);
            }
        }

        Some(())
    }
}

impl ObjectStoreStock {
    pub fn new(data: &ObjectStore) -> ObjectStoreStock {
        let mut data_static = Self::default();
        data_static.merge_from(data);
        data_static
    }

    fn merge_from(&mut self, data: &ObjectStore) {
        for object in data.objects() {
            let object = object.borrow().clone();

            self.objects.insert(object.id(), object);
        }
    }

    pub fn object(&self, id: ObjectId) -> Option<&Object> {
        self.objects.get(&id)
    }

    /// Returns the 'prototype' for this object
    /// which is the parent if its a custom object,
    /// or the original if its a stock modified object
    pub fn object_prototype(&self, object: &Object) -> Option<&Object> {
        self.objects
            .get(&object.id())
            .or_else(|| object.parent_id().and_then(|pid| self.objects.get(&pid)))
    }

    pub fn objects(&self) -> impl Iterator<Item = &Object> {
        self.objects.values()
    }
}

fn read_func_file<P: AsRef<Path>>(path: P, metadata: &MetadataStore, data: &mut ObjectStore) {
    dbg!(path.as_ref());

    let src = fs::read(path).unwrap();
    let entries = profile::Entries::new(&src);

    for entry in entries {
        data.insert_func_entry(entry, metadata);
    }
}

fn read_slk_file<P: AsRef<Path>>(
    path: P,
    kind: ObjectKind,
    metadata: &MetadataStore,
    data: &mut ObjectStore,
) {
    let src = fs::read(path).unwrap();
    let mut table = slk::Table::new(&src).unwrap();
    let legend = table.legend();

    while table.has_next() {
        if let Some(row) = table.next_row() {
            data.insert_slk_row(kind, row, &legend, metadata);
        }
    }
}

pub fn read_data_dir<P: AsRef<Path>>(path: P, metadata: &MetadataStore) -> ObjectStore {
    let path = path.as_ref();
    let mut data = ObjectStore::default();

    const SLKS: &[(ObjectKind, &str)] = &[
        (ObjectKind::UNIT, "units/unitdata.slk"),
        (ObjectKind::ABILITY, "units/abilitydata.slk"),
        (ObjectKind::ITEM, "units/itemdata.slk"),
        (ObjectKind::BUFF, "units/abilitybuffdata.slk"),
        (ObjectKind::DESTRUCTABLE, "units/destructabledata.slk"),
        (ObjectKind::UPGRADE, "units/upgradedata.slk"),
        (ObjectKind::DOODAD, "doodads/doodads.slk"),
        (ObjectKind::empty(), "units/unitbalance.slk"),
        (ObjectKind::empty(), "units/unitabilities.slk"),
        (ObjectKind::empty(), "units/unitweapons.slk"),
        (ObjectKind::empty(), "units/unitui.slk"),
    ];

    for (kind, file_path) in SLKS {
        read_slk_file(path.join(file_path), *kind, &metadata, &mut data);
    }

    const PROFILES: &[&str] = &[
        "units/campaignabilityfunc.txt",
        "units/campaignunitfunc.txt",
        "units/campaignupgradefunc.txt",
        "units/commandfunc.txt",
        "units/commonabilityfunc.txt",
        "units/humanabilityfunc.txt",
        "units/humanunitfunc.txt",
        "units/humanupgradefunc.txt",
        "units/itemabilityfunc.txt",
        "units/itemfunc.txt",
        "units/miscdata.txt",
        "units/miscgame.txt",
        "units/neutralabilityfunc.txt",
        "units/neutralunitfunc.txt",
        "units/neutralupgradefunc.txt",
        "units/nightelfabilityfunc.txt",
        "units/nightelfunitfunc.txt",
        "units/nightelfupgradefunc.txt",
        "units/orcabilityfunc.txt",
        "units/orcunitfunc.txt",
        "units/orcupgradefunc.txt",
        "units/undeadabilityfunc.txt",
        "units/undeadunitfunc.txt",
        "units/undeadupgradefunc.txt",
        "units_en/campaignabilitystrings.txt",
        "units_en/campaignunitstrings.txt",
        "units_en/campaignupgradestrings.txt",
        "units_en/commandstrings.txt",
        "units_en/commonabilitystrings.txt",
        "units_en/humanabilitystrings.txt",
        "units_en/humanunitstrings.txt",
        "units_en/humanupgradestrings.txt",
        "units_en/itemabilitystrings.txt",
        "units_en/itemstrings.txt",
        "units_en/neutralabilitystrings.txt",
        "units_en/neutralunitstrings.txt",
        "units_en/neutralupgradestrings.txt",
        "units_en/nightelfabilitystrings.txt",
        "units_en/nightelfunitstrings.txt",
        "units_en/nightelfupgradestrings.txt",
        "units_en/orcabilitystrings.txt",
        "units_en/orcunitstrings.txt",
        "units_en/orcupgradestrings.txt",
        "units_en/undeadabilitystrings.txt",
        "units_en/undeadunitstrings.txt",
        "units_en/undeadupgradestrings.txt",
        "units_en/unitglobalstrings.txt",
    ];

    for file_path in PROFILES {
        read_func_file(path.join(file_path), &metadata, &mut data);
    }

    data
}
