use ds::table::Table;

pub type TableId = usize;

pub struct Database {
    pub(crate) tables: Vec<Table>,
}
